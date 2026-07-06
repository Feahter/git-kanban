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
#[command(name = "gh-kanban", about = "Terminal kanban board for GitHub/GitLab Issues")]
struct Args {
    /// Repository in owner/name format (e.g. "owner/repo")
    #[arg(short, long)]
    repo: Option<String>,

    /// Platform: github or gitlab (default: github)
    #[arg(long, value_enum, default_value_t = types::Platform::Github)]
    platform: types::Platform,

    /// Output issues as JSON array and exit
    #[arg(long)]
    json: bool,

    /// Refresh cache and exit (for agent/cron use)
    #[arg(long)]
    refresh: bool,

    /// When used with --json or --summary: filter to a specific column by ID
    #[arg(long)]
    column: Option<String>,

    /// When used with --json: comma-separated field names to include
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
    cfg.platform = args.platform;

    if cfg.repo.is_empty() {
        eprintln!("Error: no repository configured.");
        eprintln!("Set it via --repo flag or edit ~/.config/gh-kanban/config.json");
        std::process::exit(1);
    }

    if let Err(e) = sync::check_auth(&cfg.platform) {
        eprintln!("Error: {e}");
        eprintln!("Run '{} auth login' first.", cfg.platform.cli_name());
        std::process::exit(1);
    }

    // Resolve column filter
    let target_col = args.column.as_ref().and_then(|name| {
        cfg.columns.iter().find(|c| c.id == *name)
    });

    // --- Summary mode ---
    if args.summary {
        match sync::fetch_issues(&cfg.repo, &cfg.platform) {
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
        match sync::fetch_issues(&cfg.repo, &cfg.platform) {
            Ok(issues) => {
                let filtered: Vec<types::Issue> = match target_col {
                    Some(col) => issues.into_iter().filter(|i| col.matches(i)).collect(),
                    None => issues,
                };

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
        match sync::fetch_issues(&cfg.repo, &cfg.platform) {
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

    let result = ui::run(&mut terminal, cfg.repo, cfg.columns, cfg.platform);

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
fn now_iso8601() -> String {
    let total_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_ts(total_secs)
}

/// Convert seconds since epoch to ISO 8601 UTC string.
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

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
///
/// Howard Hinnant algorithm: shifts epoch to 0000-03-01 to
/// eliminate the leap-day complexity from month computation.
fn days_to_date(days: u64) -> (u32, u32, u32) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;           // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153;                      // month phase [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1;              // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 };     // [1, 12]
    let y = y + (if m <= 2 { 1 } else { 0 });
    (y as u32, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_format_ts_epoch() {
        assert_eq!(format_ts(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_format_ts_leap_year_march() {
        // 2024-03-01 00:00:00 UTC = 19783 days after epoch
        assert_eq!(&format_ts(19783 * 86400)[..10], "2024-03-01");
    }

    #[test]
    fn test_format_ts_2000_march() {
        // 2000-03-01 (century leap year)
        assert_eq!(&format_ts(11017 * 86400)[..10], "2000-03-01");
    }

    #[test]
    fn test_format_ts_feb_non_leap() {
        // 2023-02-14
        assert_eq!(&format_ts(19402 * 86400)[..10], "2023-02-14");
    }

    #[test]
    fn test_format_ts_dec_31() {
        // 2025-12-31
        assert_eq!(&format_ts(20453 * 86400)[..10], "2025-12-31");
    }

    #[test]
    fn test_format_ts_time_component() {
        // 2023-06-15 14:30:45
        // Days: 19523, Seconds offset: 14*3600+30*60+45 = 52245
        let ts = format_ts(19523 * 86400 + 52245);
        assert_eq!(ts, "2023-06-15T14:30:45Z");
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
    fn test_select_fields_null() {
        assert_eq!(select_fields(&serde_json::json!(null), &["x".into()]), serde_json::Value::Null);
    }
}
