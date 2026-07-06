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
/// Uses the Howard Hinnant algorithm for leap-year-aware date conversion.
fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();

    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    let (year, month, day) = days_to_date(total_secs / 86400);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
///
/// Algorithm by Howard Hinnant: shifts epoch to 0000-03-01 to
/// eliminate the leap-day complexity from month computation.
fn days_to_date(days: u64) -> (u32, u32, u32) {
    // Shift epoch from 1970-01-01 to 0000-03-01
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;           // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153;                      // month phase [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1;              // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = y + (if m <= 2 { 1 } else { 0 });
    (y as u32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── now_iso8601 ──

    #[test]
    fn test_now_iso8601_format() {
        let ts = now_iso8601();
        assert!(ts.len() == 20, "Expected 20-char ISO8601, got: {ts}");
        assert!(ts.ends_with('Z'), "Expected Z suffix");
        assert_eq!(&ts[4..5], "-", "Expected dash at pos 4");
        assert_eq!(&ts[7..8], "-", "Expected dash at pos 7");
        assert_eq!(&ts[10..11], "T", "Expected T at pos 10");
    }

    #[test]
    fn test_now_iso8601_epoch() {
        // Simulate epoch: Jan 1 1970 00:00:00
        let ts = format_ts(0);
        assert_eq!(ts, "1970-01-01T00:00:00Z", "epoch failed: {ts}");
    }

    #[test]
    fn test_now_iso8601_leap_year_march() {
        // 2024 is leap. March 1 = day 61 (1-indexed: Jan 31 + Feb 29)
        // Days from 1970-01-01 to 2024-03-01:
        // 54 years * 365 + 13 leap days + 31(Jan) + 29(Feb) = 19710 + 13 + 60 = 19783
        let ts = format_ts(19783 * 86400);
        assert_eq!(&ts[0..10], "2024-03-01", "leap year march failed: {ts}");
    }

    #[test]
    fn test_now_iso8601_2000_leap() {
        // 2000 is leap (divisible by 400). March 1.
        // 1970..1999 = 30 years. Leap: 1972..1996 = 7 (but 2000? no, 2000 is in)
        // No wait: days from 1970-01-01 to 2000-03-01
        // 1970-1999: 30 years * 365 = 10950 + 7 leap days (72,76,80,84,88,92,96)
        // Jan 2000: 31 days, Feb 2000: 29 days (leap) = 60 days
        // total = 10950 + 7 + 60 = 11017 days
        let ts = format_ts(11017 * 86400);
        assert_eq!(&ts[0..10], "2000-03-01", "2000-03-01 failed: {ts}");
    }

    #[test]
    fn test_now_iso8601_non_leap_feb() {
        // 2023 not leap. Feb 14
        // 1970..2022: 53 years. Leap: 1972..2020 = 13
        // days = 53*365 + 13 + 31(Jan) + 13(Feb 14) = 19345 + 13 + 44 = 19402
        let days_since_epoch: u64 = 19402;
        let ts = format_ts(days_since_epoch * 86400);
        assert_eq!(&ts[0..10], "2023-02-14", "2023-02-14 failed: {ts}");
    }

    #[test]
    fn test_now_iso8601_dec_31() {
        // 2025-12-31. 1970..2024 = 55 years. Leap: 1972..2024 = 14 (2000 yes)
        // 2025 not leap. Dec 31 = day 365 (no leap day in 2025)
        // days = 55*365 + 14 + 364 = 20075 + 14 + 364 = 20453
        // Actually: days from 1970 to 2025-01-01 = 55*365+14 = 20089
        // Add 364 days = 20453. Let's try it.
        let ts = format_ts(20453 * 86400);
        assert_eq!(&ts[0..10], "2025-12-31", "2025-12-31 failed: {ts}");
    }

    /// Helper: format a specific unix timestamp for deterministic testing.
    fn format_ts(secs: u64) -> String {
        let secs_of_day = secs % 86400;
        let hours = secs_of_day / 3600;
        let minutes = (secs_of_day % 3600) / 60;
        let seconds = secs_of_day % 60;

        let (year, month, day) = days_to_date(secs / 86400);

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    }

    // ── select_fields ──

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

    #[test]
    fn test_select_fields_empty_list() {
        let v = serde_json::json!({"a": 1, "b": 2});
        let result = select_fields(&v, &[]);
        assert!(result.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_select_fields_nonexistent() {
        let v = serde_json::json!({"a": 1});
        let result = select_fields(&v, &["z".into()]);
        assert!(result.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_select_fields_scalar() {
        let v = serde_json::json!("hello");
        let result = select_fields(&v, &["anything".into()]);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_select_fields_null() {
        let v = serde_json::json!(null);
        let result = select_fields(&v, &["x".into()]);
        assert_eq!(result, serde_json::Value::Null);
    }
}
