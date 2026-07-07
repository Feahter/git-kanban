mod types;
mod config;
mod sync;
mod ui;

use clap::{Parser, Subcommand};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use types::Backend;

#[derive(Parser)]
#[command(
    name = "git-kanban",
    about = "Terminal kanban board for Git platforms (GitHub/GitLab)",
    version
)]
struct Cli {
    /// Repository in owner/name format
    #[arg(short, long)]
    repo: Option<String>,

    /// Use GitLab backend (glab CLI) instead of GitHub (gh CLI)
    #[arg(long)]
    gitlab: bool,

    /// Output issues as JSON and exit
    #[arg(long)]
    json: bool,

    /// Read from cache only (no network)
    #[arg(long)]
    cached: bool,

    /// Filter to a specific column by ID (e.g. "doing")
    #[arg(long)]
    column: Option<String>,

    /// Comma-separated JSON fields to include (e.g. "number,title,labels")
    #[arg(long)]
    fields: Option<String>,

    /// Output per-column issue count summary as JSON and exit
    #[arg(long)]
    summary: bool,

    /// Refresh cache and exit (for agent use)
    #[arg(long)]
    refresh: bool,

    /// Action to perform (agent mode — exits after completion)
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Subcommand)]
enum Action {
    /// Create a new issue
    Create {
        /// Issue title
        title: String,
        /// Issue body/description
        #[arg(long)]
        body: Option<String>,
        /// Labels to apply (repeatable: --label bug --label urgent)
        #[arg(long)]
        label: Vec<String>,
    },
    /// Close an issue
    Close {
        /// Issue number
        number: u64,
    },
    /// Reopen a closed issue
    Reopen {
        /// Issue number
        number: u64,
    },
    /// Add a comment to an issue
    Comment {
        /// Issue number
        number: u64,
        /// Comment body text
        #[arg(long)]
        body: String,
    },
    /// Assign yourself or another user to an issue
    Assign {
        /// Issue number
        number: u64,
        /// Username to assign (omit to assign yourself)
        #[arg(long)]
        user: Option<String>,
    },
    /// Move an issue between columns (adds and/or removes labels)
    Move {
        /// Issue number
        number: u64,
        /// Labels to add (repeatable: --add-label doing --add-label wip)
        #[arg(long)]
        add_label: Vec<String>,
        /// Labels to remove (repeatable)
        #[arg(long)]
        remove_label: Vec<String>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Load config
    let mut cfg = config::load();

    // Override repo from CLI if provided
    if let Some(repo) = &cli.repo {
        cfg.repo = repo.clone();
    }

    // Override backend from CLI if provided
    if cli.gitlab {
        cfg.backend = Backend::GitLab;
    }

    // Check if repo is set
    if cfg.repo.is_empty() {
        eprintln!("Error: no repository configured.");
        eprintln!("Set it via --repo flag or edit ~/.config/git-kanban/config.json");
        std::process::exit(1);
    }

    // Check CLI auth
    if let Err(e) = sync::check_cli_auth(cfg.backend) {
        eprintln!("Error: {}", e);
        let cmd = cfg.backend.cmd();
        eprintln!("Run '{} auth login' first.", cmd);
        std::process::exit(1);
    }

    // Execute action (agent mode) — runs first so it's explicit
    if let Some(action) = cli.action {
        match action {
            Action::Create { title, body, label } => {
                match sync::create_issue(cfg.backend, &cfg.repo, &title, body.as_deref(), &label) {
                    Ok(num) => println!("{}", num),
                    Err(e) => { eprintln!("{}", e); std::process::exit(1); }
                }
            }
            Action::Close { number } => {
                if let Err(e) = sync::close_issue(cfg.backend, &cfg.repo, number) {
                    eprintln!("{}", e); std::process::exit(1);
                }
            }
            Action::Reopen { number } => {
                if let Err(e) = sync::reopen_issue(cfg.backend, &cfg.repo, number) {
                    eprintln!("{}", e); std::process::exit(1);
                }
            }
            Action::Comment { number, body } => {
                if let Err(e) = sync::add_comment(cfg.backend, &cfg.repo, number, &body) {
                    eprintln!("{}", e); std::process::exit(1);
                }
            }
            Action::Assign { number, user } => {
                let result = match user {
                    Some(ref u) => sync::assign_user(cfg.backend, &cfg.repo, number, u),
                    None => sync::assign_self(cfg.backend, &cfg.repo, number),
                };
                if let Err(e) = result { eprintln!("{}", e); std::process::exit(1); }
            }
            Action::Move { number, add_label, remove_label } => {
                if let Err(e) = sync::move_issue(cfg.backend, &cfg.repo, number, &remove_label, &add_label) {
                    eprintln!("{}", e); std::process::exit(1);
                }
            }
        }
        return Ok(());
    }

    // Resolve issues source: cached or live fetch
    let issues = if cli.cached {
        config::read_cache()
            .ok_or_else(|| {
                eprintln!("No cache found. Run without --cached first, or use --refresh.");
                std::process::exit(1);
            })
            .unwrap()
    } else {
        match sync::fetch_issues(cfg.backend, &cfg.repo) {
            Ok(issues) => {
                config::write_cache(&issues, &chrono_now());
                issues
            }
            Err(e) => {
                if let Some(cached) = config::read_cache() {
                    eprintln!("Warning: live fetch failed ({}), using cached data", e);
                    cached
                } else {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    // Refresh mode: cache was already updated above, just confirm
    if cli.refresh {
        let count = issues.len();
        println!("Cached {} issues from {}", count, cfg.repo);
        return Ok(());
    }

    // Summary mode: per-column counts
    if cli.summary {
        let counts: Vec<serde_json::Value> = cfg.columns.iter()
            .map(|col| serde_json::json!({
                "id": col.id,
                "title": col.title,
                "count": issues.iter().filter(|i| col.matches(i)).count(),
            }))
            .collect();
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "repo": cfg.repo,
            "backend": match cfg.backend {
                Backend::GitHub => "github",
                Backend::GitLab => "gitlab",
            },
            "total": issues.len(),
            "columns": counts,
        }))
        .unwrap_or_default();
        println!("{}", json);
        return Ok(());
    }

    // Resolve column filter
    let target_col = cli.column.as_ref().and_then(|name| {
        cfg.columns.iter().find(|c| c.id == *name)
    });

    // Filter issues to the column if specified
    let filtered: Vec<types::Issue> = match target_col {
        Some(col) => issues.iter().filter(|i| col.matches(i)).cloned().collect(),
        None => issues,
    };

    // JSON mode: output filtered issues and exit
    if cli.json {
        let mut v = serde_json::json!({
            "repo": cfg.repo,
            "backend": match cfg.backend {
                Backend::GitHub => "github",
                Backend::GitLab => "gitlab",
            },
            "total": filtered.len(),
            "issues": filtered,
        });

        // Apply field selection filter
        if let Some(fields_str) = &cli.fields {
            let field_list: Vec<String> = fields_str.split(',').map(|s| s.trim().to_string()).collect();
            v = select_fields(&v, &field_list);
        }

        let json = serde_json::to_string_pretty(&v).unwrap_or_default();
        println!("{}", json);
        return Ok(());
    }

    // TUI mode (default)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = ui::run(&mut terminal, cfg.repo, cfg.backend, cfg.columns);

    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Keep only the requested fields from a JSON value (recursive for objects).
fn select_fields(value: &serde_json::Value, fields: &[String]) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .filter(|(k, _)| fields.contains(k))
                .map(|(k, v)| (k.clone(), select_fields(v, fields)))
                .collect(),
        ),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| select_fields(v, fields)).collect())
        }
        other => other.clone(),
    }
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_days = now.as_secs() / 86400;
    let time_secs = now.as_secs() % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs_remain = time_secs % 60;

    let mut y = 1970i64;
    let mut remaining = total_days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for days_in_month in &month_days {
        if remaining < *days_in_month { break; }
        remaining -= *days_in_month;
        m += 1;
    }
    let day = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m + 1, day, hours, mins, secs_remain
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ── Unit tests ──
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_fields() {
        let v = serde_json::json!({"a": 1, "b": 2, "c": 3});
        let fields = vec!["a".into(), "c".into()];
        let result = select_fields(&v, &fields);
        assert_eq!(result, serde_json::json!({"a": 1, "c": 3}));
    }

    #[test]
    fn test_select_fields_nested() {
        let v = serde_json::json!({"items": [{"a": 1, "b": 2}, {"a": 3, "b": 4}], "meta": "x"});
        let fields = vec!["items".into()];
        let result = select_fields(&v, &fields);
        assert!(result.get("meta").is_none());
        assert!(result.get("items").is_some());
    }

    #[test]
    fn test_select_fields_empty() {
        let v = serde_json::json!({"a": 1, "b": 2});
        assert!(select_fields(&v, &[]).as_object().unwrap().is_empty());
    }

    #[test]
    fn test_select_fields_scalar() {
        let v = serde_json::json!("hello");
        assert_eq!(select_fields(&v, &["x".into()]), "hello");
    }

    #[test]
    fn test_is_leap() {
        assert!(!is_leap(2023));
        assert!(is_leap(2024));
        assert!(!is_leap(2100));
        assert!(is_leap(2000));
    }
}
