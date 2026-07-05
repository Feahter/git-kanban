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
    /// Repository in owner/name format
    #[arg(short, long)]
    repo: Option<String>,

    /// Output issues as JSON and exit
    #[arg(long)]
    json: bool,

    /// Refresh cache and exit (for agent use)
    #[arg(long)]
    refresh: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Load config
    let mut cfg = config::load();

    // Override repo from CLI if provided
    if let Some(repo) = &args.repo {
        cfg.repo = repo.clone();
    }

    // Check if repo is set
    if cfg.repo.is_empty() {
        eprintln!("Error: no repository configured.");
        eprintln!("Set it via --repo flag or edit ~/.config/gh-kanban/config.json");
        std::process::exit(1);
    }

    // Check gh auth
    if let Err(e) = sync::check_gh_auth() {
        eprintln!("Error: {}", e);
        eprintln!("Run 'gh auth login' first.");
        std::process::exit(1);
    }

    // JSON mode: output issues and exit
    if args.json {
        match sync::fetch_issues(&cfg.repo) {
            Ok(issues) => {
                let json = serde_json::to_string_pretty(&serde_json::json!({
                    "repo": cfg.repo,
                    "count": issues.len(),
                    "issues": issues,
                })).unwrap_or_default();
                println!("{}", json);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Refresh mode: update cache and exit
    if args.refresh {
        match sync::fetch_issues(&cfg.repo) {
            Ok(issues) => {
                config::write_cache(&issues, &chrono_now());
                println!("Cached {} issues from {}", issues.len(), cfg.repo);
            }
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // TUI mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = ui::run(&mut terminal, cfg.repo, cfg.columns);

    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn chrono_now() -> String {
    // Simple ISO 8601 without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Convert to ISO-like string (UTC)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs_remain = time_secs % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        1970 + (days / 365) as u32, // approximate, good enough for cache timestamps
        1 + ((days % 365) / 28) as u32,
        1 + ((days % 365) % 28) as u32,
        hours,
        mins,
        secs_remain)
}
