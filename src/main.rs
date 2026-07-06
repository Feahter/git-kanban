mod types;
mod config;
mod sync;
mod ui;

use std::io;
use clap::Parser;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

#[derive(Parser)]
#[command(name = "gh-kanban", about = "Terminal kanban board for GitHub Issues")]
struct Args {
    /// Repository in owner/name format (e.g. "owner/repo")
    #[arg(short, long)]
    repo: Option<String>,

    /// Output issues as JSON array and exit
    #[arg(long)]
    json: bool,

    /// Refresh cache from GitHub and exit (for agent/cron use)
    #[arg(long)]
    refresh: bool,

    /// When used with --json or --summary: filter to a specific column by ID
    /// Column IDs: todo, doing, review, done, closed (or custom from config)
    #[arg(long)]
    column: Option<String>,

    /// When used with --json: comma-separated field names to include
    /// Available: number, title, state, labels, assignees, priority, created_at, updated_at
    #[arg(long)]
    fields: Option<String>,

    /// Output per-column issue count summary as JSON and exit
    #[arg(long)]
    summary: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Load config and resolve repo
    let mut cfg = config::load();
    if let Some(repo) = &args.repo {
        cfg.repo = repo.clone();
    }

    if cfg.repo.is_empty() {
        eprintln!("Error: no repository configured.");
        eprintln!("Set it via --repo flag or edit ~/.config/gh-kanban/config.json");
        std::process::exit(1);
    }

    if let Err(e) = sync::check_gh_auth() {
        eprintln!("Error: {e}");
        eprintln!("Run 'gh auth login' first.");
        std::process::exit(1);
    }

    // Resolve column filter
    let target_col = args.column.as_ref().and_then(|name| {
        cfg.columns.iter().find(|c| c.id == *name)
    });

    // --- Summary mode ---
    if args.summary {
        match sync::fetch_issues(&cfg.repo) {
            Ok(issues) => {
                let counts: Vec<serde_json::Value> = cfg.columns.iter()
                    .filter(|col| target_col.map_or(true, |t| col.id == t.id))
                    .map(|col| {
                        let count = issues.iter().filter(|i| col.matches(i)).count();
                        serde_json::json!({
                            "id": col.id,
                            "title": col.title,
                            "count": count,
                        })
                    })
                    .collect();
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "repo": cfg.repo,
                    "total": issues.len(),
                    "columns": counts,
                })).unwrap_or_default();
                println!("{json}");
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // --- JSON mode ---
    if args.json {
        match sync::fetch_issues(&cfg.repo) {
            Ok(issues) => {
                // Filter by column if requested
                let filtered: Vec<types::Issue> = match target_col {
                    Some(col) => issues.into_iter().filter(|i| col.matches(i)).collect(),
                    None => issues,
                };

                // Apply field selection
                let field_list: Option<Vec<String>> = args.fields.as_ref().map(|f| {
                    f.split(',').map(|s| s.trim().to_string()).collect()
                });

                let issues_json: Vec<serde_json::Value> = filtered.iter().map(|issue| {
                    let mut v = serde_json::json!(issue);
                    if let Some(fields) = &field_list {
                        v = select_fields(&v, fields);
                    }
                    v
                }).collect();

                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "repo": cfg.repo,
                    "count": filtered.len(),
                    "issues": issues_json,
                })).unwrap_or_default();
                println!("{json}");
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // --- Refresh mode ---
    if args.refresh {
        match sync::fetch_issues(&cfg.repo) {
            Ok(issues) => {
                config::write_cache(&issues, &now_iso8601());
                println!("Cached {} issues from {}", issues.len(), cfg.repo);
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // --- TUI mode ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = ui::run(&mut terminal, cfg.repo, cfg.columns);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Keep only the requested fields from a JSON value (recursive for objects).
fn select_fields(value: &serde_json::Value, fields: &[String]) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let filtered: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .filter(|(k, _)| fields.contains(k))
                .map(|(k, v)| (k.clone(), select_fields(v, fields)))
                .collect();
            serde_json::Value::Object(filtered)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| select_fields(v, fields)).collect())
        }
        other => other.clone(),
    }
}

/// Generate an ISO 8601 UTC timestamp string without chrono dependency.
///
/// Uses a leap-year-aware algorithm for correct date formatting.
fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();

    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    // Days since epoch (including leap year correction)
    let mut days = total_secs / 86400;

    // Number of leap days from 1970 to today
    let y400 = days / 146097;       // 400-year cycles
    days %= 146097;
    let y100 = days / 36524;        // 100-year cycles within the 400
    days %= 36524;
    let y4   = days / 1461;         // 4-year cycles within the 100
    days %= 1461;
    let y1   = days / 365;          // 1-year cycles within the 4
    days %= 365;

    let year = 1970 + (y400 * 400 + y100 * 100 + y4 * 4 + y1) as u32;
    // Leap year check
    let is_leap = |y: u32| (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0));
    // If we're past Feb 28 in a leap year, day-of-year is off by one
    let day_of_year = if y1 > 0 && days > 59 && is_leap(year) { days + 1 } else { days };

    let month_days: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining = day_of_year;
    let mut month = 1u32;
    for &md in &month_days {
        let bound = if month == 2 && is_leap(year) { md + 1 } else { md };
        if remaining < bound {
            break;
        }
        remaining -= bound;
        month += 1;
    }

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, remaining + 1, hours, minutes, seconds
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        // Should match ISO 8601: YYYY-MM-DDTHH:MM:SSZ
        assert!(ts.len() == 20, "Expected 20-char ISO8601, got: {ts}");
        assert!(ts.ends_with('Z'), "Expected Z suffix");
        assert_eq!(&ts[4..5], "-", "Expected dash at pos 4");
        assert_eq!(&ts[7..8], "-", "Expected dash at pos 7");
        assert_eq!(&ts[10..11], "T", "Expected T at pos 10");
    }

    #[test]
    fn test_select_fields() {
        let v = serde_json::json!({"a": 1, "b": 2, "c": 3});
        let fields = vec!["a".into(), "c".into()];
        let result = select_fields(&v, &fields);
        let obj = result.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["a"], 1);
        assert_eq!(obj["c"], 3);
        assert!(obj.get("b").is_none());
    }

    #[test]
    fn test_select_fields_nested() {
        let v = serde_json::json!({"items": [{"a": 1, "b": 2}, {"a": 3, "b": 4}]});
        let fields = vec!["items".into(), "a".into()];
        let result = select_fields(&v, &fields);
        let arr = result["items"].as_array().unwrap();
        assert_eq!(arr[0]["a"], 1);
        assert!(arr[0].get("b").is_none());
    }
}
