# git-kanban

> **Terminal kanban board for Git platforms (GitHub/GitLab).**  
> **终端看板工具，支持 GitHub 和 GitLab Issues。**
>
> 858KB single binary. <10ms startup. Zero runtime deps. Agent-friendly JSON mode.
> 858KB 单文件二进制，毫秒级启动，零运行时依赖，支持 Agent 使用的 JSON 模式。

Turn your Git platform Issues into a TUI kanban board. Move cards with `h`/`j`/`k`/`l`. Create, close, comment, assign — everything goes through `gh` or `glab` CLI, inheriting **your existing auth**.

把你的 GitHub/GitLab Issues 变成终端看板，所有操作通过现有 CLI 认证，无需额外配置。

```bash
git-kanban --repo owner/name   # TUI 模式
git-kanban --json --repo R     # Agent JSON 模式
git-kanban create "bug: ..." --body "..." --label bug  # 创建 Issue
```

---

## Quick Start / 快速开始

```bash
# Prerequisites - 安装 CLI 工具
gh auth login     # GitHub
glab auth login   # GitLab

# Install - 安装
cargo install --git https://github.com/Feahter/git-kanban

# Run - 运行
git-kanban --repo owner/name
```

---

## Keybindings / 按键

| Key 按键 | Action 动作 |
|----------|-------------|
| `h`/`l` or ←/→ | Navigate columns / 左右切换列 |
| `Tab`/`BackTab` | Navigate columns / 切换列 |
| `j`/`k` or ↑/↓ | Scroll cards / 上下滚动卡片 |
| `Enter` | Open issue in browser / 浏览器打开 |
| `n` | New issue / 新建 Issue |
| `x` | Close / reopen / 关闭或重开 |
| `m` / `M` | Move right / left / 右移或左移 |
| `c` | Add comment / 添加评论 |
| `a` | Assign yourself / 指派给自己 |
| `r` | Refresh / 刷新 |
| `?` | Help / 帮助 |
| `q` or Ctrl+C | Quit / 退出 |

---

## Agent Usage / Agent 接口

git-kanban is designed for AI agents (Claude Code, Codex, Hermes). Every TUI operation also has a CLI subcommand.

专为 AI Agent 设计，TUI 所有操作都有对应的 CLI 子命令。

```bash
# Read — 读取
git-kanban --json --repo R              # List all issues (含 body 描述)
git-kanban --refresh --repo R           # Refresh cache / 刷新缓存

# Write — 写入
git-kanban create "title" --body "desc" --label bug     # Create / 创建
git-kanban close <num>                                   # Close / 关闭
git-kanban reopen <num>                                   # Reopen / 重开
git-kanban comment <num> --body "msg"                     # Comment / 评论
git-kanban assign <num>                                   # Assign self / 指派自己
git-kanban assign <num> --user someone                    # Assign to user / 指派他人
git-kanban move <num> --add-label doing --remove-label todo   # Move / 移动列

# Example agent workflow — Agent 工作流示例：
# 1. List open issues
# 2. Assign the most urgent one to yourself
# 3. Move it to "doing" column
# 4. Add a comment
issues=$(git-kanban --json --repo R)
git-kanban assign 42 \
  && git-kanban move 42 --add-label doing --remove-label todo \
  && git-kanban comment 42 --body "Taking a look"
```

### JSON Output / JSON 输出格式

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

### Move Semantics / `move` 语义说明

`move` adds and/or removes labels — it doesn't physically drag across columns.  
Agent 调用 `move` 时需同时指定 **remove 源列标签** 和 **add 目标列标签**，这是标签增删操作而非物理拖动。

```bash
# ✅ Correct — 正确用法
git-kanban move 42 --remove-label todo --add-label doing

# ❌ Wrong (issue stays in both columns) — 错误（issue 会同时存在两列）
git-kanban move 42 --add-label doing
```

---

## GitLab

```bash
git-kanban --gitlab --repo owner/name
```

Or set `"backend": "gitlab"` in `~/.config/git-kanban/config.json`.

---

## Config / 配置

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

## Design / 架构

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
- **Write path:** CLI command → refresh cache
- **Auth:** Zero config — inherits `~/.config/gh/` or `~/.config/glab/` tokens

| Metric 指标 | Value 值 |
|-------------|----------|
| Binary size 体积 | 858 KB (single file) |
| Cold start 冷启动 | <10ms |
| Dependencies 依赖 | 4 crates |
| Async runtime | ❌ tokio |
| Embedded DB | ❌ SQLite |
| HTTP client | ❌ octocrab/reqwest |

---

## License

MIT
