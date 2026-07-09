# git-kanban

> **858KB single binary. <10ms startup. Zero runtime deps. Agent-friendly JSON mode.**
> Terminal kanban board for Git platforms (GitHub / GitLab).

```bash
git-kanban --repo owner/name           # TUI kanban board
git-kanban --json --repo R --cached    # Agent: read issues in <10ms
git-kanban create "fix: ..." --label bug  # Agent: create issue
```

[中文版](./README.zh.md)

---

## Quick Start

```bash
# Prerequisites
gh auth login     # GitHub
glab auth login   # GitLab

# Install
cargo install --git https://github.com/Feahter/git-kanban

# Run
git-kanban --repo owner/name
```

---

## Agent Usage

Every TUI operation has a corresponding CLI subcommand — designed for Claude Code, Codex, Hermes.

```bash
# Read — structured JSON output
git-kanban --json --repo R                           # All issues
git-kanban --json --repo R --cached                   # From cache (no network, <10ms)
git-kanban --json --repo R --column doing             # Filter by column
git-kanban --json --repo R --fields number,title      # Select fields (less tokens)
git-kanban --summary --repo R                         # Per-column counts
git-kanban --refresh --quiet --repo R                 # Refresh cache silently

# Write — agent-safe subcommands
git-kanban create "title" --body "desc" --label bug   # → outputs issue number
git-kanban close <num>
git-kanban reopen <num>
git-kanban comment <num> --body "msg"
git-kanban assign <num>                                # Assign self
git-kanban assign <num> --user someone                 # Assign someone else
git-kanban move <num> --add-label doing --remove-label todo

# Preview without side effects
git-kanban --dry-run move <num> --add-label doing --remove-label todo
```

### Agent workflow example

```bash
issues=$(git-kanban --json --repo R)
git-kanban assign 42 \
  && git-kanban move 42 --add-label doing --remove-label todo \
  && git-kanban comment 42 --body "Taking a look"
```

### JSON output format

```json
{
  "repo": "owner/name",
  "backend": "github",
  "count": 5,
  "issues": [
    {
      "number": 42,
      "title": "Fix login bug",
      "body": "Users cannot login with SSO...",
      "state": "Open",
      "labels": ["bug", "auth"],
      "assignees": ["fez"],
      "priority": "P0",
      "created_at": "2026-07-01T10:00:00Z",
      "updated_at": "2026-07-07T12:00:00Z"
    }
  ]
}
```

### Move semantics

`move` adds/removes labels — it doesn't physically drag cards. Always specify both source and target labels:

```bash
# ✅ Correct
git-kanban move 42 --remove-label todo --add-label doing

# ❌ Wrong (issue stays in both columns)
git-kanban move 42 --add-label doing
```

---

## Keybindings

| Key | Action |
|-----|--------|
| `h`/`l` or ←/→ | Navigate columns |
| `Tab`/`BackTab` | Navigate columns |
| `j`/`k` or ↑/↓ | Scroll cards |
| `Enter` | Open issue in browser |
| `n` | New issue |
| `x` | Close / reopen |
| `m` / `M` | Move right / left |
| `c` | Add comment |
| `a` | Assign yourself |
| `r` | Refresh |
| `?` | Help |
| `q` or Ctrl+C | Quit |

---

## GitLab

```bash
git-kanban --gitlab --repo owner/name
```

Or set `"backend": "gitlab"` in `~/.config/git-kanban/config.json`.

---

## Config

`~/.config/git-kanban/config.json`:

```json
{
  "repo": "owner/name",
  "backend": "github",
  "columns": {
    "todo": ["backlog", "triage"],
    "doing": ["wip"],
    "review": ["needs-review"]
  }
}
```

---

## Design

```
                ┌──────────────────────┐
                │   git-kanban TUI     │
                │  (ratatui + serde)   │
                ├──────────┬───────────┤
                │ JSON     │ CLI       │
                │ cache    │ wrapper   │
                │(read)    │(write)    │
                └────┬─────┴─────┬─────┘
                     │           │
              ~/.cache/    gh/glab issue
           git-kanban/    list/create/
           issues.json    close/edit/comment
```

- **Read path:** JSON cache → render → background sync
- **Write path:** CLI subcommand → refresh cache
- **Auth:** Zero config — inherits `~/.config/gh/` or `~/.config/glab/` tokens

| Metric | Value |
|--------|-------|
| Binary size | 858 KB (single file) |
| Cold start | <10ms |
| Dependencies | 4 crates |
| Async runtime | ❌ tokio |
| Embedded DB | ❌ SQLite |
| HTTP client | ❌ octocrab/reqwest |

---

## License

MIT
