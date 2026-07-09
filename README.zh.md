# git-kanban

> **858KB 单文件二进制，毫秒级启动，零运行时依赖，支持 Agent 使用的 JSON 模式。**
> 终端看板工具，支持 GitHub 和 GitLab Issues。

```bash
git-kanban --repo owner/name   # TUI 看板模式
git-kanban --json --repo R     # Agent JSON 模式
git-kanban create "bug: ..." --body "..." --label bug  # 创建 Issue
```

[English](./README.md)

---

## 快速开始

```bash
# 前置条件 - 安装 CLI 工具
gh auth login     # GitHub
glab auth login   # GitLab

# 安装
cargo install --git https://github.com/Feahter/git-kanban

# 运行
git-kanban --repo owner/name
```

---

## Agent 接口

专为 AI Agent（Claude Code、Codex、Hermes）设计，TUI 所有操作都有对应的 CLI 子命令。

```bash
# 读取 — 结构化 JSON 输出
git-kanban --json --repo R              # 全部 issues（含 body 描述）
git-kanban --json --repo R --cached      # 从缓存读取（无网络，<10ms）
git-kanban --json --repo R --column doing # 按看板列过滤
git-kanban --json --repo R --fields number,title  # 选择字段（节省 token）
git-kanban --summary --repo R            # 各列计数统计
git-kanban --refresh --quiet --repo R    # 静默刷新缓存

# 写入 — Agent 安全的子命令（无需交互）
git-kanban create "标题" --body "描述" --label bug   # → 输出 issue 编号
git-kanban close <编号>
git-kanban reopen <编号>
git-kanban comment <编号> --body "评论内容"
git-kanban assign <编号>                                 # 指派给自己
git-kanban assign <编号> --user someone                  # 指派给他人
git-kanban move <编号> --add-label doing --remove-label todo

# 预览 — 无副作用
git-kanban --dry-run move <编号> --add-label doing --remove-label todo
```

### Agent 工作流示例

```bash
# 1. 列出 open issues → 2. 指派给自己 → 3. 移到 doing → 4. 评论
issues=$(git-kanban --json --repo R)
git-kanban assign 42 \
  && git-kanban move 42 --add-label doing --remove-label todo \
  && git-kanban comment 42 --body "开始处理"
```

### JSON 输出格式

```json
{
  "repo": "owner/name",
  "backend": "github",
  "count": 5,
  "issues": [
    {
      "number": 42,
      "title": "修复登录 bug",
      "body": "用户无法通过 SSO 登录...",
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

### move 语义说明

`move` 是标签增删操作，不是物理拖动。Agent 调用时需同时指定 **移除源列标签** 和 **添加目标列标签**：

```bash
# ✅ 正确用法
git-kanban move 42 --remove-label todo --add-label doing

# ❌ 错误用法（issue 会同时存在两列）
git-kanban move 42 --add-label doing
```

---

## 按键

| 按键 | 动作 |
|------|------|
| `h`/`l` 或 ←/→ | 左右切换列 |
| `Tab`/`BackTab` | 切换列 |
| `j`/`k` 或 ↑/↓ | 上下滚动卡片 |
| `Enter` | 浏览器打开 |
| `n` | 新建 Issue |
| `x` | 关闭或重开 |
| `m` / `M` | 右移或左移 |
| `c` | 添加评论 |
| `a` | 指派给自己 |
| `r` | 刷新 |
| `?` | 帮助 |
| `q` 或 Ctrl+C | 退出 |

---

## GitLab

```bash
git-kanban --gitlab --repo owner/name
```

或在 `~/.config/git-kanban/config.json` 中设置 `"backend": "gitlab"`。

---

## 配置

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

## 架构

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

- **读取路径：** JSON 缓存 → 渲染 → 后台同步
- **写入路径：** CLI 子命令 → 刷新缓存
- **认证：** 零配置 — 继承 `~/.config/gh/` 或 `~/.config/glab/` 的 token

| 指标 | 值 |
|------|-----|
| 体积 | 858 KB（单文件） |
| 冷启动 | <10ms |
| 依赖 | 4 个 crate |
| Async runtime | ❌ tokio |
| 嵌入式数据库 | ❌ SQLite |
| HTTP 客户端 | ❌ octocrab/reqwest |

---

## 许可

MIT
