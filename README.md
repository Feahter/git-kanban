# git-kanban

> Terminal kanban board for GitHub Issues / GitLab Issues.  
> 858KB single binary. 4 crates. Zero runtime deps. Every TUI action has a CLI subcommand.

[中文版](./README.zh.md)

---

## Quick start

```bash
# 1. Prerequisites
gh auth login       # GitHub

# 2. Install
cargo install --git https://github.com/Feahter/git-kanban

# 3. Verify
git-kanban --version                 # → "git-kanban 1.0.0", exit 0
git-kanban --help                    # → subcommands + flags, exit 0

# 4. Test it
git-kanban --json --repo owner/name  # → JSON issues, exit 0
```

---

## Agent usage

### Read issues (`--json`)

```bash
git-kanban --json --repo owner/name                           # All issues
git-kanban --json --repo owner/name --column doing            # Filter by column
git-kanban --json --repo owner/name --fields number,title     # Select fields (less tokens)
git-kanban --json --repo owner/name --sort created            # Sort: created|updated
git-kanban --json --repo owner/name --search "keyword"        # Search title/body
git-kanban --json --repo owner/name --brief                   # Omit body from output
git-kanban --json --repo owner/name --cached                  # Cache only (no network, <10ms)
git-kanban --summary --repo owner/name                        # Per-column counts only
```

**→ Output:** JSON object, exit 0.

```json
{
  "repo": "owner/name",
  "backend": "github",
  "from_cache": false,
  "cached_at": "",
  "total": 3,
  "issues": [
    {
      "number": 42,
      "title": "Fix login bug",
      "body": "Users cannot login with SSO...",
      "state": "Open",
      "labels": ["bug", "p0"],
      "assignees": ["fez"],
      "priority": "P0",
      "created_at": "2026-07-01T10:00:00Z",
      "updated_at": "2026-07-07T12:00:00Z"
    }
  ]
}
```

### Write operations

Write operations output JSON + exit 0 on success, exit 1 on failure (error on stderr).

| Action | Command | Success output |
|--------|---------|---------------|
| Create | `git-kanban create "title" --label bug --body "desc"` | `{"action":"create","number":43,"ok":true}` |
| Close | `git-kanban close 42` | `{"action":"close","numbers":[42],"ok":true,"failed":[]}` |
| Reopen | `git-kanban reopen 42` | `{"action":"reopen","numbers":[42],"ok":true,"failed":[]}` |
| Comment | `git-kanban comment 42 --body "message"` | `{"action":"comment","number":42,"ok":true}` |
| Assign self | `git-kanban assign 42` | `{"action":"assign","number":42,"ok":true}` |
| Assign user | `git-kanban assign 42 --user someone` | `{"action":"assign","number":42,"ok":true}` |
| Move right | `git-kanban move 42 --add-label doing --remove-label todo` | `{"action":"move","number":42,"ok":true}` |
| Edit | `git-kanban edit 42 --title "new title" --body "new body"` | `{"action":"edit","number":42,"ok":true}` |
| Open browser | `git-kanban open 42` | `{"action":"open","number":42,"ok":true}` |
| List labels | `git-kanban labels` | `{"labels":["bug","feature",...]}` |

**Close/reopen accept comma-separated numbers:** `git-kanban close 12,15,18`

### Pipe example

```bash
issues=$(git-kanban --json --repo owner/name)
git-kanban assign 42 \
  && git-kanban move 42 --add-label doing --remove-label todo \
  && git-kanban comment 42 --body "Taking a look"
```

### Preview without side effects

```bash
git-kanban --dry-run move 42 --add-label doing --remove-label todo
```

### Cache management

```bash
git-kanban --refresh --repo owner/name     # Force re-fetch
git-kanban --refresh --quiet --repo ...    # Silent refresh
git-kanban --json --cached --repo ...      # Read cache, skip network
```

## Why Agents Love It

git-kanban was designed for agents first, humans second. Every read outputs **structured JSON** — not terminal tables, not HTML, not chatty `gh` output that costs tokens to parse. An agent consumes it as native data in one call.

The `--json --cached --fields number,title,priority` combo delivers the entire current backlog in **<10ms, zero network**. That means an agent can check its todo list, decide the next action, and execute it within the same tool invocation — no blocking, no API latency.

Every write is a deterministic CLI subcommand: JSON success/failure on stdout, errors on stderr, clean exit codes. Agents chain reads, decisions, and writes into atomic workflows. `--dry-run` previews intent without side effects. The JSON cache persists across sessions, so a stateless agent picks up exactly where the last invocation left off.

**JSON in, orders out.** git-kanban is the kanban board that speaks my language.

---

## Config

`~/.config/git-kanban/config.json` — set default repo and custom column labels:

```json
{
  "repo": "owner/name",
  "backend": "github",
  "columns": {
    "todo":    ["todo", "status:todo"],
    "doing":   ["doing", "status:doing", "in-progress"],
    "review":  ["review", "status:review"],
    "done":    ["done", "status:done"],
    "closed":  []
  }
}
```

Backend: `"github"` or `"gitlab"`. Each column maps labels → an issue with any matching label appears in that column. The `closed` column with empty labels shows all closed issues.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (auth, args, API failure) |

---

## All flags & subcommands

| Arg | Description |
|-----|-------------|
| `--repo <R>` | Repository `owner/name` (required unless in config) |
| `--json` | Output issues as JSON (read mode) |
| `--summary` | Per-column counts |
| `--column <C>` | Filter by column id |
| `--fields <F>` | Comma-separated fields: `number,title,state,labels` |
| `--sort <S>` | Sort: `created` or `updated` |
| `--search <K>` | Keyword filter (title/body, case-insensitive) |
| `--brief` | Omit body from JSON output |
| `--cached` | Cache only (no API call) |
| `--refresh` | Force cache refresh |
| `--gitlab` | Use GitLab backend |
| `--dry-run` | Preview write, no side effects |
| `--quiet` | Suppress non-essential output |
| `create` | Create issue |
| `close <N>` | Close issue(s) |
| `reopen <N>` | Reopen issue(s) |
| `comment <N>` | Add comment |
| `assign <N>` | Assign issue |
| `move <N>` | Move between columns (add/remove labels) |
| `edit <N>` | Edit title/body/labels |
| `open <N>` | Open in browser |
| `labels` | List all repo labels |

---

## TUI (keybindings)

Run `git-kanban --repo owner/name` for the terminal UI.

| Key | Action |
|-----|--------|
| `h`/`l` or ←/→ | Navigate columns |
| `Tab`/`BackTab` | Navigate columns |
| `j`/`k` or ↑/↓ | Scroll cards |
| `Enter` | Open in browser |
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
git-kanban --gitlab --repo owner/project
```

Or set `"backend": "gitlab"` in config.

---

## JSON schema

| Field | Type | Description |
|-------|------|-------------|
| `repo` | string | `"owner/name"` |
| `backend` | string | `"github"` or `"gitlab"` |
| `from_cache` | bool | Served from local cache |
| `cached_at` | string | ISO 8601 UTC of cache timestamp |
| `total` | integer | Number of issues in response |
| `issues[]` | array | Issue objects |
| `issues[].number` | integer | Issue number |
| `issues[].title` | string | Title |
| `issues[].body` | string | Body (empty string if none) |
| `issues[].state` | string | `"Open"` or `"Closed"` |
| `issues[].labels` | array | Label strings |
| `issues[].assignees` | array | User login strings |
| `issues[].priority` | string\|null | `"P0"`–`"P3"` or null |
| `issues[].created_at` | string | ISO 8601 UTC |
| `issues[].updated_at` | string | ISO 8601 UTC |

### Summary output schema

```json
{
  "repo": "owner/name",
  "backend": "github",
  "total": 3,
  "columns": [
    {"id": "todo", "title": "📋 Todo", "count": 1},
    {"id": "doing", "title": "🔧 Doing", "count": 2}
  ]
}
```

---

## Move semantics

`move` adds/removes **labels**—always specify both source and target:

```bash
git-kanban move 42 --remove-label todo --add-label doing   # ✅
git-kanban move 42 --add-label doing                        # ❌ stays in both columns
```

---

## Design

```
4 crates: ratatui, crossterm, serde, clap
No tokio  ❌    No reqwest  ❌    No SQLite  ❌    No chrono  ❌

Read:  JSON cache → render → background sync
Write: CLI subcommand → refresh cache
Auth:  Inherits ~/.config/gh/ or ~/.config/glab/ tokens
```

| Metric | Value |
|--------|-------|
| Binary | 858 KB |
| Start | <10ms |
| Dependencies | 4 crates |

---

## License

MIT
