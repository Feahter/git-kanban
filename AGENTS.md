# gh-kanban — Agent Instructions

Terminal kanban board for GitHub Issues. All operations go through `gh` CLI.

## Build & Install

```bash
cargo build --release
# Binary at target/release/gh-kanban
```

## Common Agent Workflows

### Read Issues (JSON)
```bash
# All issues (live — calls gh/glab)
gh-kanban --json --repo owner/name

# From local cache (instant, <10ms, may be stale)
gh-kanban --json --repo owner/name --cached

# All issues (GitLab)
gh-kanban --json --repo namespace/project --platform gitlab

# Filter by column
gh-kanban --json --repo owner/name --column todo

# Select fields
gh-kanban --json --repo owner/name --fields number,title,state,priority

# Per-column summary
gh-kanban --summary --repo owner/name
```

### Cache Management
```bash
# Refresh cache without TUI
gh-kanban --refresh --repo owner/name

# Cache location
# ~/.cache/gh-kanban/issues.json
```

### Write Operations (via gh CLI directly)
```bash
# Create issue
gh issue create --repo owner/name --title "Title" --label "todo"

# Close issue
gh issue close <number> --repo owner/name

# Move issue (change label)
gh issue edit <number> --repo owner/name --add-label doing --remove-label todo
```

## Architecture

```
src/
├── main.rs     — CLI entry, flag parsing, JSON/summary/refresh modes
├── types.rs    — Data types: Issue, Column, Priority, Config
├── config.rs   — Config loading, JSON cache read/write
├── sync.rs     — gh CLI wrapper (issue CRUD, label mgmt)
└── ui.rs       — ratatui TUI rendering and event handling
```

## Code Rules

- No async runtime (tokio). No HTTP client (reqwest/octocrab). No database (SQLite).
- Errors → stderr. Data → stdout.
- All write operations delegate to `gh` CLI via `std::process::Command`.
- Config → `~/.config/gh-kanban/config.json`. Cache → `~/.cache/gh-kanban/issues.json`.

## Boundaries

- **Do not add** tokio, reqwest, octocrab, SQLite, or chrono dependencies.
- **Do not modify** ui.rs unless adding TUI features.
- **Do not remove** the JSON cache — it's the primary read path for offline and fast startup.
