# Changelog

## [1.0.0] — 2026-07-13

Terminal kanban board for GitHub/GitLab Issues. Agent-first, human-second.

### Features
- TUI: 5-column kanban (todo/doing/review/done/closed) with keyboard navigation
- JSON output (`--json`): structured issue data for agents
- Read: `--column`, `--fields`, `--sort`, `--search`, `--brief`, `--cached`
- Write: `create`, `close`, `reopen`, `comment`, `assign`, `move`, `edit`, `open`
- Batch: `close`/`reopen` support comma-separated issue numbers
- Preview: `--dry-run` for all write operations
- Cache: local JSON cache at `~/.cache/git-kanban/` with `--cached`/`--refresh`
- Multi-repo: sidebar TUI + CLI multi-repo mode via `--repo`/`--repos`
- GitLab: `--gitlab` flag for GitLab backend
- Agent-friendly: all actions exposed as CLI subcommands with JSON responses

### Tech
- Rust binary, ~1MB release, 4 crates (ratatui, crossterm, serde, clap)
- Zero async runtime, zero HTTP client, zero SQLite
- All external operations via `gh`/`glab` CLI
