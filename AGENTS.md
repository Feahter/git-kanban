# git-kanban — Agent Instructions

Terminal kanban board for GitHub/GitLab Issues. All operations go through `gh` or `glab` CLI.

## Build & Install

```bash
cargo build --release
# Binary at target/release/git-kanban
```

## Common Agent Workflows

### Read Issues (JSON)
```bash
# All issues (live — calls gh/glab)
git-kanban --json --repo owner/name

# From local cache (instant, <10ms, may be stale)
git-kanban --json --repo owner/name --cached

# All issues (GitLab)
git-kanban --json --repo namespace/project --platform gitlab

# Filter by column
git-kanban --json --repo owner/name --column todo

# Select fields
git-kanban --json --repo owner/name --fields number,title,state,priority

# Per-column summary
git-kanban --summary --repo owner/name

# Filter summary by column
git-kanban --summary --repo owner/name --column doing
```

### Write Operations (built-in subcommands)
```bash
# Create issue
git-kanban create "Title" --repo owner/name --label todo --body "Description"

# Close issue
git-kanban close <number> --repo owner/name

# Reopen issue
git-kanban reopen <number> --repo owner/name

# Add comment
git-kanban comment <number> --repo owner/name --body "Comment text"

# Assign self
git-kanban assign <number> --repo owner/name

# Assign someone else
git-kanban assign <number> --repo owner/name --user someone

# Move issue (change labels)
git-kanban move <number> --repo owner/name --add-label doing --remove-label todo
```

### Cache Management
```bash
# Refresh cache without TUI
git-kanban --refresh --repo owner/name

# Cache location
# ~/.cache/git-kanban/issues.json
```

### TUI Mode (default)
```bash
git-kanban --repo owner/name
```

## Architecture

```
src/
├── main.rs     — CLI entry, flag parsing, JSON/summary/refresh modes, write commands
├── types.rs    — Data types: Issue, Column, Priority, Config
├── config.rs   — Config loading, JSON cache read/write
├── sync.rs     — gh/glab CLI wrapper (issue CRUD, label mgmt)
└── ui.rs       — ratatui TUI rendering and event handling
```

## Code Rules

- No async runtime (tokio). No HTTP client (reqwest/octocrab). No database (SQLite).
- Errors → stderr. Data → stdout.
- All external operations delegate to `gh` or `glab` CLI via `std::process::Command`.
- Config → `~/.config/git-kanban/config.json`. Cache → `~/.cache/git-kanban/issues.json`.

## Boundaries

- **Do not add** tokio, reqwest, octocrab, SQLite, or chrono dependencies.
- **Do not modify** ui.rs unless adding TUI features.
- **Do not remove** the JSON cache — it's the primary read path for offline and fast startup.
