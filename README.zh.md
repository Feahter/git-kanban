# git-kanban

> 终端看板工具，支持 GitHub/GitLab Issues。
> 858KB 单文件二进制，4 个依赖 crate。每个 TUI 操作都有对应的 CLI 子命令（Agent 友好）。

[English](./README.md)

---

## 快速开始

```bash
# 1. 前置条件
gh auth login       # GitHub

# 2. 安装
cargo install --git https://github.com/Feahter/git-kanban

# 3. 验证
git-kanban --version                 # → "git-kanban 1.0.0", exit 0
git-kanban --help                    # → 子命令 + 参数列表, exit 0

# 4. 测试
git-kanban --json --repo owner/name  # → JSON issues, exit 0
```

---

## Agent 用法

### 读取（`--json`）

```bash
git-kanban --json --repo owner/name                        # 全部 issues
git-kanban --json --repo owner/name --column doing          # 按列过滤
git-kanban --json --repo owner/name --fields number,title   # 选字段（省 token）
git-kanban --json --repo owner/name --sort created          # 排序: created|updated
git-kanban --json --repo owner/name --search "关键词"        # 搜索标题/正文
git-kanban --json --repo owner/name --brief                 # 省略 body
git-kanban --json --repo owner/name --cached                # 缓存读取（无网络，<10ms）
git-kanban --summary --repo owner/name                      # 各列计数
```

**→ 输出:** JSON 对象, exit 0。

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
      "title": "修复登录 bug",
      "body": "用户无法通过 SSO 登录...",
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

### 写入操作

写入操作输出 JSON。exit 0 = 成功, exit 1 = 失败（错误信息在 stderr）。

| 操作 | 命令 | 成功输出 |
|------|------|---------|
| 创建 | `git-kanban create "标题" --label bug --body "描述"` | `{"action":"create","number":43,"ok":true}` |
| 关闭 | `git-kanban close 42` | `{"action":"close","numbers":[42],"ok":true,"failed":[]}` |
| 重开 | `git-kanban reopen 42` | `{"action":"reopen","numbers":[42],"ok":true,"failed":[]}` |
| 评论 | `git-kanban comment 42 --body "消息"` | `{"action":"comment","number":42,"ok":true}` |
| 指派自己 | `git-kanban assign 42` | `{"action":"assign","number":42,"ok":true}` |
| 指派他人 | `git-kanban assign 42 --user someone` | `{"action":"assign","number":42,"ok":true}` |
| 右移 | `git-kanban move 42 --add-label doing --remove-label todo` | `{"action":"move","number":42,"ok":true}` |
| 编辑 | `git-kanban edit 42 --title "新标题" --body "新内容"` | `{"action":"edit","number":42,"ok":true}` |
| 浏览器打开 | `git-kanban open 42` | `{"action":"open","number":42,"ok":true}` |
| 列出标签 | `git-kanban labels` | `{"labels":["bug","feature",...]}` |

**关闭/重开支持逗号分隔编号：** `git-kanban close 12,15,18`

### 管道示例

```bash
issues=$(git-kanban --json --repo owner/name)
git-kanban assign 42 \
  && git-kanban move 42 --add-label doing --remove-label todo \
  && git-kanban comment 42 --body "开始处理"
```

### 预览（无副作用）

```bash
git-kanban --dry-run move 42 --add-label doing --remove-label todo
```

### 缓存管理

```bash
git-kanban --refresh --repo owner/name     # 强制重新拉取
git-kanban --refresh --quiet --repo ...    # 静默刷新
git-kanban --json --cached --repo ...      # 仅读缓存，不走网络
```

---

## 配置

`~/.config/git-kanban/config.json` — 设置默认 repo 和自定义列映射：

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

后端：`"github"` 或 `"gitlab"`。每列通过标签匹配 issue——issue 有任意一个匹配标签即出现在该列。`closed` 列的标签为空数组，显示所有已关闭的 issue。

---

## 退出码

| 码 | 含义 |
|----|------|
| 0 | 成功 |
| 1 | 错误（认证/参数/API 失败） |

---

## 全部参数 & 子命令

| 参数 | 说明 |
|------|------|
| `--repo <R>` | 仓库 `owner/name`（配置文件中已设时可省略） |
| `--json` | 以 JSON 输出 issue 列表（读取模式） |
| `--summary` | 各列计数统计 |
| `--column <C>` | 按列 ID 过滤 |
| `--fields <F>` | 逗号分隔字段：`number,title,state,labels` |
| `--sort <S>` | 排序：`created` 或 `updated` |
| `--search <K>` | 关键词过滤（标题/正文，大小写不敏感） |
| `--brief` | JSON 输出中省略 body |
| `--cached` | 仅读缓存，不走 API |
| `--refresh` | 强制刷新缓存 |
| `--gitlab` | 使用 GitLab 后端 |
| `--dry-run` | 预览写入操作，无副作用 |
| `--quiet` | 压制非必要输出 |
| `create` | 创建 issue |
| `close <N>` | 关闭 issue |
| `reopen <N>` | 重开 issue |
| `comment <N>` | 添加评论 |
| `assign <N>` | 指派 issue |
| `move <N>` | 移动列（增删标签） |
| `edit <N>` | 编辑标题/正文/标签 |
| `open <N>` | 浏览器打开 |
| `labels` | 列出仓库所有标签 |

---

## TUI 快捷键

运行 `git-kanban --repo owner/name` 启动终端界面。

| 按键 | 动作 |
|------|------|
| `h`/`l` 或 ←/→ | 左右切换列 |
| `Tab`/`BackTab` | 切换列 |
| `j`/`k` 或 ↑/↓ | 上下滚动卡片 |
| `Enter` | 浏览器打开 |
| `n` | 新建 issue |
| `x` | 关闭/重开 |
| `m` / `M` | 右移/左移 |
| `c` | 添加评论 |
| `a` | 指派给自己 |
| `r` | 刷新 |
| `?` | 帮助 |
| `q` 或 Ctrl+C | 退出 |

---

## GitLab

```bash
git-kanban --gitlab --repo owner/project
```

或在配置文件中设置 `"backend": "gitlab"`。

---

## JSON schema

| 字段 | 类型 | 说明 |
|------|------|------|
| `repo` | string | `"owner/name"` |
| `backend` | string | `"github"` 或 `"gitlab"` |
| `from_cache` | bool | 是否来自本地缓存 |
| `cached_at` | string | 缓存的 ISO 8601 UTC 时间戳 |
| `total` | integer | 本次返回的 issue 数 |
| `issues[]` | array | Issue 对象数组 |
| `issues[].number` | integer | Issue 编号 |
| `issues[].title` | string | 标题 |
| `issues[].body` | string | 正文（空字符串时省略） |
| `issues[].state` | string | `"Open"` 或 `"Closed"` |
| `issues[].labels` | array | 标签字符串 |
| `issues[].assignees` | array | 指派用户登录名 |
| `issues[].priority` | string\|null | `"P0"`–`"P3"` 或 null |
| `issues[].created_at` | string | ISO 8601 UTC |
| `issues[].updated_at` | string | ISO 8601 UTC |

### 统计输出格式

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

## Move 语义

`move` 是标签操作，不是物理拖动。必须同时指定源列标签和目标列标签：

```bash
git-kanban move 42 --remove-label todo --add-label doing   # ✅
git-kanban move 42 --add-label doing                        # ❌ issue 会在两列同时出现
```

---

## 架构

```
4 个 crate: ratatui, crossterm, serde, clap
无 tokio  ❌    无 reqwest  ❌    无 SQLite  ❌    无 chrono  ❌

读取： JSON 缓存 → 渲染 → 后台同步
写入： CLI 子命令 → 刷新缓存
认证： 继承 ~/.config/gh/ 或 ~/.config/glab/ 的 token
```

| 指标 | 值 |
|------|-----|
| 二进制体积 | 858 KB |
| 冷启动 | <10ms |
| 依赖 | 4 crate |

---

## 许可

MIT
