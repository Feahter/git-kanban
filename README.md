# gh-kanban

> **Terminal kanban board for GitHub Issues.**  
> 858KB single binary. <10ms startup. Zero runtime deps. Agent-friendly JSON mode.

Turn your GitHub Issues into a TUI kanban board. Move cards between columns with `h`/`j`/`k`/`l`. Create, close, refresh — every operation goes through `gh` CLI, inheriting **your existing auth**. No daemon, no database, no web server, no config hell.

```
cargo install --git https://github.com/Feahter/gh-kanban
gh-kanban --repo owner/name
```

---

## Why

GitHub Issues is where your work lives, but its web UI is slow and modal-ridden.  
You don't need Jira. You need `gh issue list --json` rendered into columns you can move cards across with keystrokes.

**gh-kanban is that.** The laziest, fastest wrapper around `gh` that gives you a visual kanban without leaving your terminal.

---

## Features

### 🏠 Fully Local

- Every keystroke hits **your JSON cache**, not GitHub's servers
- Network only touches `gh` CLI — **your auth, your rate limit, your proxy config**
- Offline? Reads last cached state instantly
- No daemon, no web server, no Electron, no "sign in with GitHub" popup

### ⚡ Ridiculously Fast

| Metric | Value |
|--------|-------|
| Binary size | **858 KB** (single file, `strip`+`LTO`) |
| Cold start | **<10ms** from JSON cache |
| Dependencies | **4 crates** — ratatui, crossterm, serde_json, clap |
| No async runtime | ❌ tokio |
| No embedded DB | ❌ SQLite |
| No HTTP client | ❌ octocrab/reqwest |
| No system deps | ❌ ncurses, libsqlite3, libssl |

### 🤖 Agent-Friendly

Built for Hermes / Claude Code / Codex agents:

```bash
# JSON output — pipe into any agent
gh-kanban --json --repo owner/name

# Refresh cache silently (for cron/agent workflows)
gh-kanban --refresh --repo owner/name

# Agent reads: {"repo":"feZ/repo","count":42,"issues":[...]}
```

Every TUI action has a corresponding `gh` CLI call, so agents can script the same operations.

---

## Quick Start

### Prerequisites

```bash
# GitHub CLI installed and authenticated
gh auth login
```

### Install

```bash
cargo install --git https://github.com/Feahter/gh-kanban
```

Or download the binary from [Releases](https://github.com/Feahter/gh-kanban/releases).

### Run

```bash
# First run — point at your repo
gh-kanban --repo Feahter/gh-kanban

# Config saved to ~/.config/gh-kanban/config.json — subsequent runs remember it
gh-kanban
```

---

## Usage

### Keybindings

| Key | Action |
|-----|--------|
| `h`/`l` or ←/→ | Navigate columns |
| `Tab`/`BackTab` | Navigate columns |
| `j`/`k` or ↑/↓ | Scroll cards |
| `Enter` | Open issue in browser |
| `n` | New issue (prompts for title) |
| `x` | Close issue |
| `m` | Move to next column |
| `r` | Refresh from GitHub |
| `?` | Show help |
| `q` or Ctrl+C | Quit |

### Labels = Columns

Issues organize into columns by their **labels**. Default mapping:

| Column | Matches labels |
|--------|---------------|
| 📋 Todo | `todo`, `status:todo` |
| 🔧 Doing | `doing`, `status:doing`, `in-progress` |
| 👀 Review | `review`, `status:review` |
| ✅ Done | `done`, `status:done` |
| ❌ Closed | state=closed (any label) |

Priority detected from labels: `P0`, `P1`, `P2`, `P3` or `priority:0`–`priority:3`.

### Customize Columns

Edit `~/.config/gh-kanban/config.json` to override column label mappings:

```json
{
  "repo": "owner/name",
  "columns": {
    "todo": ["backlog", "triage"],
    "doing": ["wip", "in-progress"],
    "review": ["needs-review"]
  }
}
```

---

## Design

```
                ┌──────────────────────┐
                │   gh-kanban TUI      │
                │  (ratatui + serde)   │
                ├──────────┬───────────┤
                │ JSON     │ gh CLI    │
                │ cache    │ wrapper   │
                │(read)    │(write)    │
                └────┬─────┴─────┬─────┘
                     │           │
              ~/.cache/    gh issue list
           gh-kanban/    gh issue create
           issues.json   gh issue close
                         gh issue edit
```

- **Read path:** JSON cache → render instantly → background sync refreshes cache
- **Write path:** `gh issue create/edit/close` → refresh cache on completion
- **Auth:** Zero config — inherits your existing `~/.config/gh/` OAuth token

---

## Roadmap

- [x] Kanban columns with label mapping
- [x] Priority coloring (P0–P3)
- [x] New / close / move issues
- [x] Agent JSON mode
- [x] Offline cache
- [ ] Search / filter (`/` key)
- [ ] Issue detail view (body, comments)
- [ ] Mouse drag-and-drop
- [ ] Multi-repo support

---

## License

MIT
