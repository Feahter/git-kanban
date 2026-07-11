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

    /// Multiple repositories (repeatable: --repos owner/a --repos owner/b)
    #[arg(long)]
    repos: Vec<String>,

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

    /// Preview the action without executing (dry run)
    #[arg(long)]
    dry_run: bool,

    /// Filter issues by keyword in title or body (case-insensitive)
    #[arg(long)]
    search: Option<String>,

    /// Sort output by field: created or updated
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(["created", "updated"]))]
    sort: Option<String>,

    /// Omit issue body from JSON output (shorter)
    #[arg(long)]
    brief: bool,

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
    /// Close an issue (supports comma-separated: "12,15,18")
    Close {
        /// Issue number(s), comma-separated
        number: String,
    },
    /// Reopen a closed issue (supports comma-separated)
    Reopen {
        /// Issue number(s), comma-separated
        number: String,
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
    /// Open an issue in the browser
    Open {
        /// Issue number
        number: u64,
    },
    /// Edit an issue (title, body, or labels)
    Edit {
        /// Issue number
        number: u64,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New body/description
        #[arg(long)]
        body: Option<String>,
        /// Labels to add (repeatable)
        #[arg(long)]
        add_label: Vec<String>,
        /// Labels to remove (repeatable)
        #[arg(long)]
        remove_label: Vec<String>,
    },
    /// List all labels in the repository
    Labels,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Load config
    let mut cfg = config::load();

    // Resolve repo list: CLI repos > CLI repo > config repos > config repo
    let repos: Vec<String> = if !cli.repos.is_empty() {
        cli.repos.clone()
    } else if let Some(ref repo) = cli.repo {
        vec![repo.clone()]
    } else {
        cfg.repo_list()
    };

    // Override repo from CLI if provided
    if let Some(repo) = &cli.repo {
        cfg.repo = repo.clone();
    }

    // Override backend from CLI if provided
    if cli.gitlab {
        cfg.backend = Backend::GitLab;
    }

    // Check if repo is set
    if repos.is_empty() && cfg.repo.is_empty() {
        eprintln!("Error: no repository configured.");
        eprintln!("Set it via --repo flag, --repos flags, or edit ~/.config/git-kanban/config.json");
        std::process::exit(1);
    }
    // Use first repo for auth check and as default action target
    let _first_repo = repos.first().cloned().unwrap_or_else(|| cfg.repo.clone());

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
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"create","title":title,"labels":label,"dry_run":true}));
                    return Ok(());
                }
                match sync::create_issue(cfg.backend, &cfg.repo, &title, body.as_deref(), &label) {
                    Ok(num) => {
                        if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                            if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                                eprintln!("Warning: failed to write cache");
                            }
                        }
                        println!("{}", serde_json::json!({"action":"create","number":num,"ok":true}));
                    }
                    Err(e) => { eprintln!("{}", e); std::process::exit(1); }
                }
            }
            Action::Close { number } => {
                let nums: Vec<u64> = number.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if nums.is_empty() {
                    eprintln!("Error: invalid issue number(s): '{}'", number);
                    std::process::exit(1);
                }
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"close","numbers":nums,"dry_run":true}));
                    return Ok(());
                }
                let mut failed = Vec::new();
                for n in &nums {
                    if let Err(e) = sync::close_issue(cfg.backend, &cfg.repo, *n) {
                        eprintln!("Error closing #{}: {}", n, e);
                        failed.push(*n);
                    }
                }
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"close","numbers":nums,"ok":failed.is_empty(),"failed":failed}));
                if !failed.is_empty() { std::process::exit(1); }
            }
            Action::Reopen { number } => {
                let nums: Vec<u64> = number.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                if nums.is_empty() {
                    eprintln!("Error: invalid issue number(s): '{}'", number);
                    std::process::exit(1);
                }
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"reopen","numbers":nums,"dry_run":true}));
                    return Ok(());
                }
                let mut failed = Vec::new();
                for n in &nums {
                    if let Err(e) = sync::reopen_issue(cfg.backend, &cfg.repo, *n) {
                        eprintln!("Error reopening #{}: {}", n, e);
                        failed.push(*n);
                    }
                }
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"reopen","numbers":nums,"ok":failed.is_empty(),"failed":failed}));
                if !failed.is_empty() { std::process::exit(1); }
            }
            Action::Comment { number, body } => {
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"comment","number":number,"body":body,"dry_run":true}));
                    return Ok(());
                }
                if let Err(e) = sync::add_comment(cfg.backend, &cfg.repo, number, &body) {
                    eprintln!("{}", e); std::process::exit(1);
                }
                // Refresh cache after write
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"comment","number":number,"ok":true}));
            }
            Action::Assign { number, user } => {
                if cli.dry_run {
                    let target = user.as_deref().unwrap_or("self");
                    println!("{}", serde_json::json!({"action":"assign","number":number,"user":target,"dry_run":true}));
                    return Ok(());
                }
                let result = match user {
                    Some(ref u) => sync::assign_user(cfg.backend, &cfg.repo, number, u),
                    None => sync::assign_self(cfg.backend, &cfg.repo, number),
                };
                if let Err(e) = result { eprintln!("{}", e); std::process::exit(1); }
                // Refresh cache after write
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"assign","number":number,"ok":true}));
            }
            Action::Move { number, add_label, remove_label } => {
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"move","number":number,"add_label":add_label,"remove_label":remove_label,"dry_run":true}));
                    return Ok(());
                }
                if let Err(e) = sync::move_issue(cfg.backend, &cfg.repo, number, &remove_label, &add_label) {
                    eprintln!("{}", e); std::process::exit(1);
                }
                // Refresh cache after write
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"move","number":number,"ok":true}));
            }
            Action::Open { number } => {
                sync::open_in_browser(cfg.backend, &cfg.repo, number);
                println!("{}", serde_json::json!({"action":"open","number":number,"ok":true}));
            }
            Action::Edit { number, title, body, add_label, remove_label } => {
                if cli.dry_run {
                    println!("{}", serde_json::json!({"action":"edit","number":number,"title":title,"body":body,"add_label":add_label,"remove_label":remove_label,"dry_run":true}));
                    return Ok(());
                }
                if let Err(e) = sync::edit_issue(cfg.backend, &cfg.repo, number, title.as_deref(), body.as_deref(), &add_label, &remove_label) {
                    eprintln!("{}", e); std::process::exit(1);
                }
                if let Ok(issues) = sync::fetch_issues(cfg.backend, &cfg.repo) {
                    if !config::write_cache(&issues, &chrono_now(), &cfg.repo) {
                        eprintln!("Warning: failed to write cache");
                    }
                }
                println!("{}", serde_json::json!({"action":"edit","number":number,"ok":true}));
            }
            Action::Labels => {
                let labels = sync::list_labels(cfg.backend, &cfg.repo);
                match labels {
                    Ok(labels) => {
                        let json = match serde_json::to_string_pretty(&serde_json::json!({"labels": labels})) {
                            Ok(s) => s,
                            Err(e) => format!("{{\"error\":\"serialization failed: {}\"}}", e),
                        };
                        println!("{}", json);
                    }
                    Err(e) => { eprintln!("{}", e); std::process::exit(1); }
                }
            }
        }
        return Ok(());
    }

    // Resolve issues source: cached or live fetch
    let (issues, from_cache, cached_at) = if cli.cached {
        match config::read_cache_meta(&cfg.repo) {
            Some((issues, last_sync)) => (issues, true, last_sync),
            None => {
                eprintln!("No cache found. Run without --cached first, or use --refresh.");
                std::process::exit(1);
            }
        }
    } else {
        match sync::fetch_issues(cfg.backend, &cfg.repo) {
            Ok(issues) => {
                let now = chrono_now();
                if !config::write_cache(&issues, &now, &cfg.repo) {
                    eprintln!("Warning: failed to write cache");
                }
                (issues, false, now)
            }
            Err(e) => {
                if let Some((cached, last_sync)) = config::read_cache_meta(&cfg.repo) {
                    eprintln!("Warning: live fetch failed ({}), using cached data", e);
                    (cached, true, last_sync)
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
        let target_col = cli.column.as_ref().and_then(|name| {
            cfg.columns.iter().find(|c| c.id == *name)
        });
        let cols_iter: Box<dyn Iterator<Item = &types::Column>> = match &target_col {
            Some(col) => Box::new(std::iter::once(*col)),
            None => Box::new(cfg.columns.iter()),
        };
        let total_count: usize = cols_iter
            .map(|col| issues.iter().filter(|i| col.matches(i)).count())
            .sum();
        let cols_iter2: Box<dyn Iterator<Item = &types::Column>> = match &target_col {
            Some(col) => Box::new(std::iter::once(*col)),
            None => Box::new(cfg.columns.iter()),
        };
        let counts: Vec<serde_json::Value> = cols_iter2
            .map(|col| serde_json::json!({
                "id": col.id,
                "title": col.title,
                "count": issues.iter().filter(|i| col.matches(i)).count(),
            }))
            .collect();
        let json = match serde_json::to_string_pretty(&serde_json::json!({
            "repo": cfg.repo,
            "backend": match cfg.backend {
                Backend::GitHub => "github",
                Backend::GitLab => "gitlab",
            },
            "total": if cli.column.is_some() { total_count as u64 } else { issues.len() as u64 },
            "columns": counts,
        })) {
            Ok(s) => s,
            Err(e) => format!("{{\"error\":\"serialization failed: {}\"}}", e),
        };
        println!("{}", json);
        return Ok(());
    }

    // Resolve column filter
    let target_col = cli.column.as_ref().and_then(|name| {
        cfg.columns.iter().find(|c| c.id == *name)
    });

    // Filter issues to the column if specified
    let mut filtered: Vec<types::Issue> = match target_col {
        Some(col) => issues.iter().filter(|i| col.matches(i)).cloned().collect(),
        None => issues,
    };

    // Search filter: keyword in title or body (case-insensitive)
    if let Some(keyword) = &cli.search {
        let kw = keyword.to_lowercase();
        filtered.retain(|i| i.title.to_lowercase().contains(&kw) || i.body.to_lowercase().contains(&kw));
    }

    // Sort if requested
    if let Some(sort_field) = &cli.sort {
        match sort_field.as_str() {
            "created" => filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            "updated" => filtered.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            _ => {}
        }
    }

    // JSON mode: output filtered issues and exit
    if cli.json {
        let mut v = serde_json::json!({
            "repo": cfg.repo,
            "backend": match cfg.backend {
                Backend::GitHub => "github",
                Backend::GitLab => "gitlab",
            },
            "from_cache": from_cache,
            "cached_at": cached_at,
            "total": filtered.len(),
            "issues": filtered,
        });

        // Apply field selection filter — only to issues, not root
        if let Some(fields_str) = &cli.fields {
            let field_list: Vec<String> = fields_str.split(',').map(|s| s.trim().to_string()).collect();
            if let Some(issues_arr) = v["issues"].as_array_mut() {
                *issues_arr = issues_arr.iter().map(|i| select_fields(i, &field_list)).collect();
            }
        }

        // Apply --brief: remove body
        if cli.brief {
            if let Some(issues_arr) = v["issues"].as_array_mut() {
                for issue in issues_arr.iter_mut() {
                    if let Some(obj) = issue.as_object_mut() {
                        obj.remove("body");
                    }
                }
            }
        }

        let json = match serde_json::to_string_pretty(&v) {
            Ok(s) => s,
            Err(e) => format!("{{\"error\":\"serialization failed: {}\"}}", e),
        };
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

    // ── CLI argument parsing tests ──

    #[test]
    fn test_cli_gitlab_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--gitlab", "--repo", "user/repo"]).unwrap();
        assert!(cli.gitlab);
        assert_eq!(cli.repo.as_deref(), Some("user/repo"));
    }

    #[test]
    fn test_cli_default_no_gitlab() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "user/repo"]).unwrap();
        assert!(!cli.gitlab);
    }

    #[test]
    fn test_cli_cached_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--cached", "--repo", "u/r"]).unwrap();
        assert!(cli.cached);
    }

    #[test]
    fn test_cli_column_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--column", "doing", "--repo", "u/r"]).unwrap();
        assert_eq!(cli.column.as_deref(), Some("doing"));
    }

    #[test]
    fn test_cli_fields_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--fields", "number,title", "--repo", "u/r"]).unwrap();
        assert_eq!(cli.fields.as_deref(), Some("number,title"));
    }

    #[test]
    fn test_cli_summary_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--summary", "--repo", "u/r"]).unwrap();
        assert!(cli.summary);
    }

    #[test]
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--json", "--repo", "u/r"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_cli_refresh_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--refresh", "--repo", "u/r"]).unwrap();
        assert!(cli.refresh);
    }

    // ── Subcommand tests ──

    #[test]
    fn test_cli_create_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "create", "My Title"]).unwrap();
        match cli.action.unwrap() {
            Action::Create { title, body, label } => {
                assert_eq!(title, "My Title");
                assert!(body.is_none());
                assert!(label.is_empty());
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn test_cli_create_with_labels() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "create", "Bug fix", "--label", "bug", "--label", "urgent",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Create { title, body, label } => {
                assert_eq!(title, "Bug fix");
                assert_eq!(label, vec!["bug", "urgent"]);
                assert!(body.is_none());
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn test_cli_create_with_body() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "create", "My Title", "--body", "Description text",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Create { title, body, label } => {
                assert_eq!(title, "My Title");
                assert_eq!(body.as_deref(), Some("Description text"));
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn test_cli_close_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "close", "42"]).unwrap();
        match cli.action.unwrap() {
            Action::Close { number } => assert_eq!(number, "42"),
            _ => panic!("expected Close action"),
        }
    }

    #[test]
    fn test_cli_reopen_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "reopen", "7"]).unwrap();
        match cli.action.unwrap() {
            Action::Reopen { number } => assert_eq!(number, "7"),
            _ => panic!("expected Reopen action"),
        }
    }

    #[test]
    fn test_cli_comment_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "comment", "10", "--body", "nice work"]).unwrap();
        match cli.action.unwrap() {
            Action::Comment { number, body } => {
                assert_eq!(number, 10);
                assert_eq!(body, "nice work");
            }
            _ => panic!("expected Comment action"),
        }
    }

    #[test]
    fn test_cli_assign_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "assign", "42"]).unwrap();
        match cli.action.unwrap() {
            Action::Assign { number, user } => {
                assert_eq!(number, 42);
                assert!(user.is_none());
            }
            _ => panic!("expected Assign action"),
        }
    }

    #[test]
    fn test_cli_assign_with_user() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "assign", "42", "--user", "someone"]).unwrap();
        match cli.action.unwrap() {
            Action::Assign { number, user } => {
                assert_eq!(number, 42);
                assert_eq!(user.as_deref(), Some("someone"));
            }
            _ => panic!("expected Assign action"),
        }
    }

    #[test]
    fn test_cli_move_subcommand() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "move", "42", "--add-label", "doing", "--remove-label", "todo",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Move { number, add_label, remove_label } => {
                assert_eq!(number, 42);
                assert_eq!(add_label, vec!["doing"]);
                assert_eq!(remove_label, vec!["todo"]);
            }
            _ => panic!("expected Move action"),
        }
    }

    #[test]
    fn test_cli_move_multiple_labels() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "move", "42",
            "--add-label", "doing", "--add-label", "wip",
            "--remove-label", "todo", "--remove-label", "backlog",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Move { number, add_label, remove_label } => {
                assert_eq!(number, 42);
                assert_eq!(add_label, vec!["doing", "wip"]);
                assert_eq!(remove_label, vec!["todo", "backlog"]);
            }
            _ => panic!("expected Move action"),
        }
    }

    #[test]
    fn test_cli_create_with_body_and_labels() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "create", "Feature", "--body", "Description", "--label", "enhancement",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Create { title, body, label } => {
                assert_eq!(title, "Feature");
                assert_eq!(body.as_deref(), Some("Description"));
                assert_eq!(label, vec!["enhancement"]);
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn test_cli_comment_requires_body() {
        let result = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "comment", "10"]);
        assert!(result.is_err(), "comment subcommand should require --body");
    }

    #[test]
    fn test_chrono_now_format() {
        let result = chrono_now();
        // ISO 8601: 2024-01-15T12:30:00Z (always 20 chars with zero-padding)
        assert_eq!(result.len(), 20, "expected 20 chars, got {}: {}", result.len(), result);
        assert!(result.ends_with('Z'), "expected Z suffix");
        assert_eq!(&result[4..5], "-", "expected dash after year");
        assert_eq!(&result[7..8], "-", "expected dash after month");
        assert_eq!(&result[10..11], "T", "expected T separator");
        assert_eq!(&result[13..14], ":", "expected colon after hours");
        assert_eq!(&result[16..17], ":", "expected colon after minutes");
    }

    #[test]
    fn test_cli_parse_without_repo() {
        // --repo is optional at parse time (Option<String>); error is runtime
        let cli = Cli::try_parse_from(&["git-kanban", "--json"]).unwrap();
        assert!(cli.repo.is_none());
        assert!(cli.json);
    }

    #[test]
    fn test_cli_parse_json_with_column() {
        let cli = Cli::try_parse_from(&["git-kanban", "--json", "--column", "doing", "--repo", "u/r"]).unwrap();
        assert!(cli.json);
        assert_eq!(cli.column.as_deref(), Some("doing"));
        assert_eq!(cli.repo.as_deref(), Some("u/r"));
    }

    #[test]
    fn test_cli_parse_summary_with_column() {
        let cli = Cli::try_parse_from(&["git-kanban", "--summary", "--column", "todo", "--repo", "u/r"]).unwrap();
        assert!(cli.summary);
        assert_eq!(cli.column.as_deref(), Some("todo"));
    }

    #[test]
    fn test_cli_parse_json_fields_with_column() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--json", "--fields", "number,title",
            "--column", "doing", "--repo", "u/r",
        ]).unwrap();
        assert!(cli.json);
        assert_eq!(cli.fields.as_deref(), Some("number,title"));
        assert_eq!(cli.column.as_deref(), Some("doing"));
    }

    #[test]
    fn test_cli_parse_refresh_cached_conflict() {
        // Both --refresh and --cached are flags; they can be set together
        // at parse time (runtime chooses based on order)
        let cli = Cli::try_parse_from(&["git-kanban", "--refresh", "--cached", "--repo", "u/r"]).unwrap();
        assert!(cli.refresh);
        assert!(cli.cached);
    }

    #[test]
    fn test_cli_parse_create_with_multiple_labels() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "create", "New Feature", "--label", "enhancement",
            "--label", "feature", "--body", "A new feature",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Create { title, body, label } => {
                assert_eq!(title, "New Feature");
                assert_eq!(body.as_deref(), Some("A new feature"));
                assert_eq!(label, vec!["enhancement", "feature"]);
            }
            _ => panic!("expected Create action"),
        }
    }

    #[test]
    fn test_cli_parse_move_no_labels() {
        // Move with no --add-label and no --remove-label is valid but degenerate
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "move", "42"]).unwrap();
        match cli.action.unwrap() {
            Action::Move { number, add_label, remove_label } => {
                assert_eq!(number, 42);
                assert!(add_label.is_empty());
                assert!(remove_label.is_empty());
            }
            _ => panic!("expected Move action"),
        }
    }

    // ── Additional subcommand tests ──

    #[test]
    fn test_cli_open_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "open", "42"]).unwrap();
        match cli.action.unwrap() {
            Action::Open { number } => assert_eq!(number, 42),
            _ => panic!("expected Open action"),
        }
    }

    #[test]
    fn test_cli_edit_subcommand() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "edit", "42", "--title", "New Title", "--body", "New body",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Edit { number, title, body, .. } => {
                assert_eq!(number, 42);
                assert_eq!(title.as_deref(), Some("New Title"));
                assert_eq!(body.as_deref(), Some("New body"));
            }
            _ => panic!("expected Edit action"),
        }
    }

    #[test]
    fn test_cli_edit_with_labels() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--repo", "u/r",
            "edit", "42",
            "--add-label", "bug", "--remove-label", "feature",
        ]).unwrap();
        match cli.action.unwrap() {
            Action::Edit { number, add_label, remove_label, .. } => {
                assert_eq!(number, 42);
                assert_eq!(add_label, vec!["bug"]);
                assert_eq!(remove_label, vec!["feature"]);
            }
            _ => panic!("expected Edit action"),
        }
    }

    #[test]
    fn test_cli_labels_subcommand() {
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "labels"]).unwrap();
        match cli.action.unwrap() {
            Action::Labels => {} // just parsing succeeds
            _ => panic!("expected Labels action"),
        }
    }

    // ── New flags ──

    #[test]
    fn test_cli_dry_run_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--dry-run", "--repo", "u/r", "close", "42"]).unwrap();
        assert!(cli.dry_run);
    }

    #[test]
    fn test_cli_search_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--search", "bug", "--repo", "u/r"]).unwrap();
        assert_eq!(cli.search.as_deref(), Some("bug"));
    }

    #[test]
    fn test_cli_sort_created() {
        let cli = Cli::try_parse_from(&["git-kanban", "--sort", "created", "--repo", "u/r"]).unwrap();
        assert_eq!(cli.sort.as_deref(), Some("created"));
    }

    #[test]
    fn test_cli_sort_updated() {
        let cli = Cli::try_parse_from(&["git-kanban", "--sort", "updated", "--repo", "u/r"]).unwrap();
        assert_eq!(cli.sort.as_deref(), Some("updated"));
    }

    #[test]
    fn test_cli_sort_invalid() {
        let result = Cli::try_parse_from(&["git-kanban", "--sort", "invalid", "--repo", "u/r"]);
        assert!(result.is_err(), "invalid sort value should fail to parse");
    }

    #[test]
    fn test_cli_brief_flag() {
        let cli = Cli::try_parse_from(&["git-kanban", "--brief", "--repo", "u/r"]).unwrap();
        assert!(cli.brief);
    }

    // ── Combined flags ──

    #[test]
    fn test_cli_combined_json_brief_fields() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--json", "--brief",
            "--fields", "number,title",
            "--column", "doing",
            "--repo", "u/r",
        ]).unwrap();
        assert!(cli.json);
        assert!(cli.brief);
        assert_eq!(cli.fields.as_deref(), Some("number,title"));
        assert_eq!(cli.column.as_deref(), Some("doing"));
    }

    #[test]
    fn test_cli_combined_summary_column_search() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--summary", "--column", "todo",
            "--search", "urgent", "--sort", "updated",
            "--repo", "u/r",
        ]).unwrap();
        assert!(cli.summary);
        assert_eq!(cli.column.as_deref(), Some("todo"));
        assert_eq!(cli.search.as_deref(), Some("urgent"));
        assert_eq!(cli.sort.as_deref(), Some("updated"));
    }

    #[test]
    fn test_cli_refresh_dry_run() {
        let cli = Cli::try_parse_from(&[
            "git-kanban", "--refresh", "--dry-run", "--repo", "u/r",
        ]).unwrap();
        assert!(cli.refresh);
        assert!(cli.dry_run);
    }

    // ── Error cases ──

    #[test]
    fn test_cli_close_requires_number() {
        // close subcommand requires a positional number
        let result = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "close"]);
        assert!(result.is_err(), "close requires a number argument");
    }

    #[test]
    fn test_cli_reopen_invalid_number_type() {
        // number is a String, so any string parses fine; validation happens at runtime
        let cli = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "reopen", "abc"]).unwrap();
        match cli.action.unwrap() {
            Action::Reopen { number } => assert_eq!(number, "abc"),
            _ => panic!("expected Reopen action"),
        }
    }

    #[test]
    fn test_cli_unknown_subcommand() {
        let result = Cli::try_parse_from(&["git-kanban", "--repo", "u/r", "unknown-cmd"]);
        assert!(result.is_err(), "unknown subcommand should fail to parse");
    }
}
