# gh-kanban

Terminal kanban board for GitHub Issues — built with Rust + ratatui.

## Installation

```bash
cargo install --git https://github.com/Feahter/gh-kanban
```

### Prerequisites

- GitHub CLI (`gh`) installed and authenticated: `gh auth login`

## Usage

```bash
# First run generates config, then set your repo
gh-kanban --repo owner/name

# Or edit ~/.config/gh-kanban/config.json
{
  "repo": "owner/name"
}

# JSON output (for agent integration)
gh-kanban --json --repo owner/name

# Refresh cache without TUI
gh-kanban --refresh --repo owner/name
```

### Keybindings

| Key | Action |
|-----|--------|
| `h`/`l` or ←/→ | Navigate columns |
| `Tab`/`BackTab` | Navigate columns |
| `j`/`k` or ↑/↓ | Scroll cards |
| `Enter` | Open issue in browser |
| `n` | New issue |
| `x` | Close issue |
| `m` | Move to next column |
| `r` | Refresh |
| `?` | Help |
| `q` or Ctrl+C | Quit |

### How columns work

Issues are organized into columns based on their **labels**. The default config has:

| Column | Matches labels |
|--------|---------------|
| 📋 Todo | `todo`, `status:todo` |
| 🔧 Doing | `doing`, `status:doing`, `in-progress` |
| 👀 Review | `review`, `status:review` |
| ✅ Done | `done`, `status:done` |
| ❌ Closed | (state=closed) |

Priority is detected from labels `P0`-`P3` or `priority:0`-`priority:3`.

## Design

- **~8MB** single binary, no runtime dependencies
- **<10ms** startup from JSON cache
- **gh CLI** for all GitHub operations (inherits your auth)
- **JSON cache** at `~/.cache/gh-kanban/issues.json`
- No async runtime, no SQLite, no HTTP library — just `std::process::Command` + `serde_json`

## License

MIT
