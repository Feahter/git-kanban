# 多 Repo 侧边栏设计 — 审校报告

> 模式C：基于摘要推理的架构审校
> 审校时间：2026-07-11
> 审校对象：`git-kanban` 多 repo 侧边栏设计方案

---

## 一、遗漏的边缘情况

### E1. `--repo` 与 `--repos` 冲突未定义

> **风险**: 高 | **影响**: CLI 行为不确定

设计说同时支持 `--repo` 和 `--repos`，但没定义两者同时出现时谁优先。

- 如果用户写 `--repo owner/a --repos owner/b --repos owner/c`
  → 应该合并为 3 个 repo，还是忽略 --repo？
- 建议规则：`--repos` 优先（显式多 repo 意图），`--repo` 只在没有 `--repos` 时生效。

### E2. `repos` 数组为空的兜底未定义

> **风险**: 中 | **影响**: 用户困惑

如果用户配置了 `"repos": []` 且没有 `"repo"` 老字段 → 进入 "no repository configured" 错误。但如果 `"repos": []` 且 `"repo": "foo/bar"` 呢？应该降级为单 repo 模式，但当前设计没提。

### E3. 各 repo 可拥有不同的 backend

> **风险**: 低（设计假设合理）| **影响**: 需文档化

设计隐含所有 repo 使用同一个 backend。但如果用户需要同时跟踪 GitHub 和 GitLab 的项目，当前设计不支持。这不是 bug，但应该在文档中明确声明为限制。

### E4. 多 repo 下的 `--column` / `--fields` / `--search` / `--sort` 语义

> **风险**: 中 | **影响**: CLI 模式行为不完整

这些过滤器目前对单 repo 的 issues 数组应用。多 repo 时：
- `--column` 应该应用到每个 repo 的 issues 上，还是只过滤总输出？
- `--search` 是全局搜还是按 repo 搜？
- `--sort` 是全局排序还是每个 repo 内排序？
- 最合理的做法：每个 repo 独立过滤+排序，与单 repo 行为一致。

### E5. TUI 中写操作后的跨 repo 缓存影响

> **风险**: 中 | **影响**: 用户感知数据不一致

在 TUI 中执行 `n`（创建）、`x`（关闭）、`m`（移动）、`c`（评论）、`a`（分配）后，代码只刷新**当前 repo** 的缓存。如果用户切换到另一个 repo，它的缓存可能已经陈旧。这不是问题——但**如果**用户预期 `r` 刷新当前 repo 后其他 repo 也自动刷新，则需要 `R`（Shift-r）全量刷新功能，设计未提及。

### E6. `--refresh` 和 `--summary` 在多 repo 模式下的输出

> **风险**: 中 | **影响**: CLI 工具契约变更

- `--refresh` 目前输出 `"Cached 42 issues from owner/repo"`。多 repo 时应输出每个 repo 的缓存状态。
- `--summary` 目前输出单 repo 的列计数。多 repo 时应叠加所有 repo 的计数，还是输出分 repo 的摘要？

### E7. TUI 侧边栏宽度固定 20 字符的硬编码风险

> **风险**: 低 | **影响**: 长 repo 名被裁剪

repo 名为 `very-long-organization-name/repository-name` 时 20 字符不够。需截断逻辑（如中间 `…`）。

---

## 二、Config 向后兼容深度分析

### 现状

```rust
pub struct Config {
    pub repo: String,       // 单 repo
    pub backend: Backend,
    pub columns: Vec<Column>,
}
```

### 目标

```rust
pub struct Config {
    pub repo: String,            // 保持向后兼容的旧字段
    pub repos: Vec<String>,      // 新增多 repo 字段
    pub backend: Backend,
    pub columns: Vec<Column>,
}
```

### 序列化坑

| 场景 | 旧 config.json | 应该怎么处理 |
|------|----------------|-------------|
| 只有老的 `"repo"` | `{"repo": "a/b"}` | `repos = []`, `repo = "a/b"` → 单 repo 模式，不显示侧边栏 |
| 新的 `"repos"` + 老的 `"repo"` | `{"repo": "a/b", "repos": ["c/d", "e/f"]}` | 以 `repos` 为主，`repo` 只做兼容读取 |
| 只有新 `"repos"` | `{"repos": ["a/b", "c/d"]}` | `repos = ["a/b", "c/d"]`, `repo = "a/b"`（取第一个） |
| 老的 `"repo"` + 空的 `"repos"` | `{"repo": "a/b", "repos": []}` | 视为单 repo 模式 |

### 反序列化方案（推荐）

```rust
// 先读 repos，如果存在且非空，用它
// 否则 fallback 到 repo
// repos 为空但 repo 非空 → 单 repo 模式
// repos 为空且 repo 为空 → 无 repo，报错
```

### 写 config.json 问题

当前 `load()` **不写** config——它只读。所以新的 Config 增加 `repos` 字段不需要写回。但是要注意：如果用户通过 `--repos` 传入多 repo，这些不会自动持久化到 config.json。设计需要决定是否要支持 `--save-config` 或 `--repos-save` 来持久化 CLI 参数。

---

## 三、TUI 多 repo 切换时的状态管理

### 当前状态矩阵

```
ui::run() 栈上:
  selected_row: usize       ← 每列内的行选择

App 结构体:
  selected_col: usize       ← 选中的列
  repo: String              ← 当前活跃 repo
  columns: Vec<Column>      ← 当前 repo 的看板数据
  status_msg: String
  loading: bool
```

### 本地变量 + App 字段分离的问题

当前 `selected_row` 是 `ui::run()` 的局部变量——它和 `selected_col` 跨域在不同生命周期上。这样当 `App` 被重新渲染时，`selected_row` 需要手动跟踪。虽然当前的 ratatui 模式没问题（draw 闭包捕获它），但多 repo 后，**每个 repo 需要记住自己的 `selected_row` 和 `selected_col`**。

### 建议的新状态结构

```rust
struct App {
    repos: Vec<String>,            // 所有配置 repo
    repo_selected: usize,          // 侧边栏选中哪个 repo（按索引）
    focus: Focus,                  // Sidebar | Kanban
    per_repo_state: Vec<RepoState>, // 每个 repo 记住自己的看板状态
    repo: String,                  // 当前活跃 repo（与 repos[repo_selected] 对应）
    backend: Backend,
    columns: Vec<Column>,         // 当前 repo 的列数据
    selected_col: usize,
    status_msg: String,
    loading: bool,
}

struct RepoState {
    columns: Vec<Column>,  // 缓存每个 repo 的看板数据
    selected_col: usize,
    // selected_row 保持为 run() 的局部变量，因为它是每帧绘制的
}
```

### 切换 repo 时的生命周期

```
Tab → focus = Sidebar
j/k → repo_selected += 1 / -= 1  (在 focus=Sidebar 时)
Tab → focus = Kanban, 从 per_repo_state[repo_selected] 恢复 columns 和 selected_col
      selected_row = 0 (重置行选择)
```

**关键决策点**: 是否每个 repo 记住自己的行列状态？还是切换时一律重置？

- 如果记忆：用户频繁切换时不丢失浏览位置，体验更好
- 如果重置：实现更简单，但用户会觉得"白切了"
- **建议**: 每个 repo 缓存列数据 + selected_col；selected_row 不缓存（切换 repo 意味着看板内容大变，行位置无意义）

---

## 四、缓存一致性分析

### 当前写入路径

```
任何一个写操作（create/close/move/comment/assign）后:
  1. sync::fetch_issues(backend, repo)  → 拉取最新
  2. config::write_cache(&issues, &now, &repo)  → 写入 issues-{repo}.json
```

### 多 repo 缓存文件布局（已天然支持）

```
~/.cache/git-kanban/
  issues-owner-a.json    ← 每个 repo 独立缓存
  issues-owner-b.json    ← 天然隔离，无竞争条件
  issues-owner-c.json
```

`cache_file_path` 已经把 `/` 转成 `-`，所以 `issues-owner-name.json` 格式已经存在。

### 一致性风险

| 场景 | 风险 | 缓解 |
|------|------|------|
| 在 TUI 中对 repo A 写操作 | 无：A 的缓存会刷新 | 已有的 refresh-after-write 模式 |
| 在 CLI 中对 repo A 写操作（agent 模式） | 无：A 的缓存会刷新 | 同上 |
| 在 repo B 上用 `--cached` 时 repo A 被外部修改 | **有**：B 的缓存是 A 在另一个时间点的快照 | 这不是问题——只是显示"陈旧数据" |
| 用户从 repo A 切换到 repo B | **有**：B 的缓存可能数小时未更新 | 建议：切换到 B 时，"静默后台刷新"（显示 `loading=true`，如果失败则使用缓存） |
| 全量刷新所有 repo | 设计缺失 | 需要 `R` 键或 `--refresh-all` 标志 |

### 建议

- 切换 repo 时：尝试 live fetch，失败 fallback 到缓存（目前单 repo 已有的 fallback 模式，扩展到每个 repo）
- 保留 `r` 刷新当前 repo
- 新增 `R`（Shift-r）刷新所有 repo（后台串行，显示进度）

---

## 五、CLI 多 repo JSON 输出格式设计

### 单 repo（现有，保持不动）

```json
{
  "repo": "owner/name",
  "backend": "github",
  "from_cache": false,
  "cached_at": "2026-01-01T12:00:00Z",
  "total": 42,
  "issues": [...]
}
```

### 多 repo（建议的新格式）

```json
{
  "repos": true,
  "total": 85,
  "repositories": [
    {
      "repo": "owner/a",
      "backend": "github",
      "from_cache": false,
      "cached_at": "2026-01-01T12:00:00Z",
      "total": 42,
      "issues": [...]
    },
    {
      "repo": "owner/b",
      "backend": "github",
      "from_cache": false,
      "cached_at": "2026-01-01T12:05:00Z",
      "total": 43,
      "issues": [...]
    }
  ]
}
```

### 破坏性变更提醒

**所有现有的 agent 脚本**如果解析 `result["repo"]` 或 `result["issues"]`，在多 repo 模式下会拿到 `undefined`/`null`。

**缓解方案**：
1. 只在确实有多个 repo 时使用新格式（设计已提出这一条）
2. 单 repo 时 `--json` 输出完全不变
3. 在 README/AGENTS.md 中**明确标注**这个破坏性变更

### --summary 格式（多 repo）

```json
{
  "repos": true,
  "repositories": [
    {
      "repo": "owner/a",
      "total": 42,
      "columns": [...]
    },
    ...
  ],
  "aggregate": {
    "total": 85,
    "columns": [
      {"id": "todo", "count": 20},
      ...
    ]
  }
}
```

### --refresh 输出（多 repo）

```
Cached 42 issues from owner/a
Cached 43 issues from owner/b
Cached  0 issues from owner/c
Refreshed 3 repositories
```

---

## 六、实现顺序和依赖关系

```
Phase 1: 类型系统扩展 (types.rs)
  ↓
Phase 2: Config 加载扩展 (config.rs)
  ↓
Phase 3: CLI 参数+路由 (main.rs)  ← 这一步之后 CLI 多 repo 就基本可用
  ↓
Phase 4: TUI 侧边栏 (ui.rs)       ← 最大的改动量
  ↓
Phase 5: 缓存全量刷新 (main.rs + ui.rs)
  ↓
Phase 6: 测试适配 (config tests + cli_test.rs)
```

| Phase | 文件 | 改动量 | 估算 | 前置依赖 |
|-------|------|--------|------|----------|
| 1 | `src/types.rs` | ~10 行新增 + ~5 行修改 | 0.5h | 无 |
| 2 | `src/config.rs` | ~30 行修改（load() 逻辑） + 测试 | 1h | Phase 1 |
| 3a | `src/main.rs` | 加 `--repos` 参数 (~5 行) | 0.5h | Phase 2 |
| 3b | `src/main.rs` | 多 repo 路由逻辑 (JSON/summary/refresh/action 路径) | 2h | Phase 3a |
| 4a | `src/ui.rs` | App 结构体扩展 + RepoState | 1h | Phase 1 |
| 4b | `src/ui.rs` | 侧边栏渲染（draw 函数） | 2h | Phase 4a |
| 4c | `src/ui.rs` | 焦点切换 + 键盘事件（Tab/j/k） | 2h | Phase 4a |
| 4d | `src/ui.rs` | 写操作分发到当前 repo | 1h | Phase 4c |
| 5 | `src/main.rs` + `src/ui.rs` | `R` 全量刷新 | 1h | Phase 4 |
| 6a | `tests/config_test.rs` (已内联) | repos 加载测试 | 0.5h | Phase 2 |
| 6b | `tests/cli_test.rs` | JSON 格式测试 + `--repos` flag 测试 | 1h | Phase 3b |
| 总 | — | — | **~11.5h** | — |

---

## 七、现有测试改动清单

### config.rs 单元测试（新增 ~6 个）

| 测试名 | 描述 | 重要性 |
|--------|------|--------|
| `test_load_with_repos_array` | `{"repos": ["a/b", "c/d"]}` → `cfg.repos.len() == 2` | **关键** |
| `test_load_repos_backward_compat` | `{"repo": "a/b"}` → `cfg.repos` 为空，`cfg.repo == "a/b"` | **关键** |
| `test_load_repos_with_repo_fallback` | `{"repo": "a/b", "repos": []}` → `cfg.repos` 为空，`cfg.repo == "a/b"` | 重要 |
| `test_load_empty_config_no_repos_no_repo` | `{}` → `cfg.repos` 为空，`cfg.repo` 为空 | 重要 |
| `test_load_repos_backend_per_repo` | 验证 backend 对所有 repo 一致（当前设计假设） | 可选 |
| `test_cache_isolation` | 写入 `owner/a` 不影响 `owner/b` 的缓存 | 重要 |

### cli_test.rs 集成测试（新增 ~4 个，修改 ~1 个）

| 测试名 | 描述 | 重要性 |
|--------|------|--------|
| `test_help_mentions_repos_flag` | `--help` 应包含 `--repos` | **关键** |
| `test_repos_flag_json_output` | `--repos A --repos B --json` 验证新格式 | **关键** |
| `test_repos_flag_single_repo_no_sidebar` | 单 repo 配置 → 侧边栏不显示 | 重要 |
| **修改** `test_json_output_structure` | 当 `--repo` 单 repo 时保持旧格式验证 | **关键**（不能破坏） |

---

## 八、风险摘要

| # | 风险 | 级别 | 缓解 |
|---|------|------|------|
| R1 | `--repo` 与 `--repos` 未定义冲突 | 🔴 高 | 明确定义优先级规则 |
| R2 | JSON 输出格式破坏现有 agent 脚本 | 🔴 高 | 单 repo 保持旧格式不变量 |
| R3 | 切换 repo 时的状态重置 vs 记忆 | 🟡 中 | 缓存列数据+选中列；不缓存行位置 |
| R4 | 切换 repo 时是否静默 live fetch | 🟡 中 | 做静默后台刷新，失败 fallback 到缓存 |
| R5 | `selected_row` 当前是局部变量 | 🟡 中 | 保持它在 `ui::run()` 栈上，切换 repo 时重置 |
| R6 | 缺少全量刷新所有 repo 的机制 | 🟢 低 | 在 Phase 5 中加 `R` 键 |
| R7 | 侧边栏 20 字符对长 repo 名不够 | 🟢 低 | 加截断逻辑 |
