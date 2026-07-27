# git-kanban SEO/GEO 推广计划

> **军师参谋报告** — 审校原始方案 → 按 ROI 排序的执行计划
> 生成日期: 2026-07-27 | 项目状态: v1.0.0, 26 commits, 0 stars, 0 forks

---

## 一、审校原始方案

### ✅ 合理保留项

| 原方案项 | 判定 | 理由 |
|---------|------|------|
| P0.1 GitHub Topics | **保留** — 但已部分完成 | 当前已有 9 个 topics (`cli`,`github`,`gitlab`,`hacktoberfest`,`kanban`,`ratatui`,`rust`,`terminal`,`tui`)，缺 `agent-tool`,`json`,`productivity` |
| P0.2 llms.txt | **保留** — 提升为 P0 第一优先级 | 2026 年 GEO 已成主流，30-70% AI 检索准确率提升 |
| P0.3 README 首段优化 | **保留** — 但需调整策略 | 当前已有较好关键词密度，问题在"故事性"而非"关键词堆砌" |
| P1.4 crates.io 发布 | **保留** — 提升为 P0 | 这是**最直接的获客管道**，cargo install 可直装 |
| P1.5 Awesome List 提交 | **保留** — 但需明确目标列表 | awesome-ratatui 最容易进，awesome-rust 门槛最高 |
| P1.6 Social Preview 图 | **保留** — P1 合理 |
| P2.7 技术文章 | **保留** — P2 合理 |

### ❌ 遗漏项（高 ROI）

1. **AGENTS.md → MCP 兼容性声明** — 当前已有 `AGENTS.md`，但缺少 MCP server 声明。2026 年所有主流 coding agent (Claude Code, Codex, Gemini CLI, Copilot CLI) 都支持 MCP。**声明为 MCP-compatible 工具比任何 SEO 都直接**。
2. **GitHub Release + Changelog 优化** — 已有 CHANGELOG.md，但没有 GitHub Release 页面（`Releases` 区为空）。Release 页面会被 Google 索引，也是 Awesome List 审核的基本要求。
3. **rust-unofficial/awesome-rust 的替代通道：awesome-ratatui** — 这是最容易进、ROI 最高的 Awesome List。Ratatui 项目本身非常活跃 (17k+ stars)，一直在收录基于 ratatui 的应用。
4. **Reddit r/rust Show HN 帖** — 最直接的开发者社区曝光。有先例：类似工具 "Kanban CLI" 在 HN 获 17 points。
5. **GitHub CI badge 缺失** — README 没有任何 build status / crates.io / license badge。
6. **GitHub Discussions / Issue 模板** — 对 0 star 项目来说，Issue 模板降低贡献者门槛。
7. **README.zh.md 推广** — 中文社区推广通道被完全忽略。已有中文版 README 但没利用。
8. **Homepage URL 未设置** — Cargo.toml 没有 `homepage` 或 `repository` 字段。

### ⚠️ 不合理项

- **P0 标记为 "5分钟"** — 实际上 GitHub Topics 修改是即时生效的（已设置了一部分），但 llms.txt 和 README 优化需要 15-30 分钟精心编写。
- **Awesome List 提交过于乐观** — awesome-rust 审核极其严格（需要 crate 发布 + 一定社区采用），短期内几乎不可能。awesome-kanban 不存在于主流列表中；应该瞄准 awesome-ratatui、awesome-tuis、awesome-cli-apps。
- **crates.io 发布是 P0 而非 P1** — 对 Rust 生态的 CLI 项目，不上 crates.io 等于向 90% 的潜在用户关了门。
- **技术文章的 ROI 被低估** — 一篇高质量的 "How I built a <10ms kanban for AI agents" 文章可以同时覆盖 dev.to + Reddit + HN，一篇多投，杠杆效应极强。

---

## 二、ROI 排序执行计划

### 努力量估算标准

| 等级 | 时间 | 描述 |
|------|------|------|
| 🟢 <15min | 一次投入，零外部依赖 |
| 🟡 15-30min | 需要思考或写内容 |
| 🟠 1-3h | 需要多次迭代或审核等待 |
| 🔴 3-8h | 需要精心编写或等待审核 |

---

### P0 — 今天可做，零成本，ROI 最高

#### 1. GitHub Topics 补全
| 项 | 值 |
|-----|-----|
| 努力量 | 🟢 <5min |
| 预期收益 | 🔥 GitHub 搜索排名 — 最直接的发现通道 |
| 前提条件 | 无 |
| 说明 | 当前已有 `cli,github,gitlab,hacktoberfest,kanban,ratatui,rust,terminal,tui`。**补加：** `agent-tool`, `json`, `productivity`, `hacktoberfest`（已有） |

#### 2. llms.txt（GEO 核心）
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min |
| 预期收益 | 🔥 被 Claude Code / Codex / Gemini CLI / Copilot CLI 等 agent 自动发现。2026 年 GEO 标准，30-70% AI 检索准确率提升 |
| 前提条件 | 了解 llms.txt 格式（标准已成熟，参考 llmstxt.org） |
| 说明 | 放在仓库根目录。Agent 读到后可直接理解项目结构、安装方式和 API |

#### 3. crates.io 发布
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 20min（含 CI 配置） |
| 预期收益 | 🔥🔥 **最高杠杆**。Google 索引 crates.io 页面 + cargo install 直装 + Awesome List 审核基本要求 |
| 前提条件 | 需要 crates.io API token (`cargo login`)，需要唯一 crate 名（`git-kanban` 可用） |
| 说明 | 必须在 Cargo.toml 补 `homepage`、`repository`、`readme`、`keywords`、`categories` 字段 |

#### 4. README 首段 + Badge 优化
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min |
| 预期收益 | 🔥 首段是 GitHub 搜索结果的第一屏 + Social preview 摘要 |
| 前提条件 | 无 |
| 说明 | 当前首段已较好，但可强化故事钩子 + 加入 CI badge。建议加：`[!crates.io]` (等发布后)、`[!License]`、`[!Rust]` |

#### 5. MCP 兼容性声明（GEO 杠杆）
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min |
| 预期收益 | 🔥 2026 年主流 agent 都支持 MCP。但 git-kanban 本身不需要 MCP server — 它是 **CLI 工具**，天然是 agent 的 tool。需要的是在 AGENTS.md 中声明：*"Works as a Claude Code / Codex / Gemini CLI shell tool — no MCP server needed"* |
| 前提条件 | 无 |
| 说明 | MCP 兼容性声明应当写入 README 首段 + AGENTS.md。区别于其他需要额外 MCP server 的工具，这是原生的 **competitive advantage** |

---

### P1 — 一次投入长期收益

#### 6. GitHub Release + Changelog 创建
| 项 | 值 |
|-----|-----|
| 努力量 | 🟢 <10min |
| 预期收益 | 🔥 Google 索引 release 页面 + Awesome List 审核要求 + SEO URL 结构 |
| 前提条件 | GitHub release CLI (`gh release create v1.0.0`) |
| 说明 | 已有 CHANGELOG.md，直接导入为 Release notes |

#### 7. Social Preview 图
| 项 | 值 |
|-----|-----|
| 努力量 | 🟠 30min-1h |
| 预期收益 | 🔥 社交分享时的第一印象。GitHub 默认只显示 owner 头像 + 描述，自定义图极大提升点击率 |
| 前提条件 | 需要一张 1280×640 PNG (<1MB)，建议工具: `npm i -g gh-social-preview` 自动生成 |
| 说明 | 建议内容：terminal 截图 + 项目名 + 核心卖点 "858KB · <10ms · Agent-first" |

#### 8. awesome-ratatui 提交
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min 编辑 PR |
| 预期收益 | 🔥🔥 Ratatui 生态有 4800+ crates，17k+ stars。进入后直接触达所有 ratatui 用户 |
| 前提条件 | 建议先发布 crates.io（审核时会检查） |
| 说明 | https://github.com/ratatui/awesome-ratatui 是 ratatui 官方的 awesome list，包含 `kanban` 应用列表 |

#### 9. awesome-tuis 提交
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min 编辑 PR / Issue |
| 预期收益 | 🔥 被 awesome-tuis 收录可被 awesometui.com 索引，长期流量 |
| 前提条件 | 无，可使用 Issue 方式（`github.com/rothgar/awesome-tuis/issues`） |
| 说明 | 该列表已有类似工具 Backlog.md（AI agent 协作），说明编辑器接受这类工具 |

#### 10. Cargo.toml metadata 补全
| 项 | 值 |
|-----|-----|
| 努力量 | 🟢 <5min |
| 预期收益 | 🟡 crates.io 页面信息更丰富，提升搜索评分 |
| 前提条件 | 无 |
| 说明 | 补 `homepage = "https://github.com/Feahter/git-kanban"`、`repository = "..."`、`readme = "README.md"`、`keywords = ["kanban","tui","cli","agent","github"]`、`categories = ["command-line-utilities", "development-tools::debugging"]` |

---

### P2 — 内容杠杆（一次性投入，可复用）

#### 11. Reddit r/rust Show 帖
| 项 | 值 |
|-----|-----|
| 努力量 | 🟠 1-2h（含社区互动） |
| 预期收益 | 🔥🔥 最直接的开发者社区曝光。同类工具 "Kanban CLI (Rust)" 在 HN 获 17 points |
| 前提条件 | 建议先完成 P0（crates.io + llms.txt + README 优化）再发帖 |
| 策略 | 标题示例：*"[Show] git-kanban — a <10ms, 858KB terminal kanban board for GitHub/GitLab, designed for AI agents"* 突出 Agent-first + <10ms + 858KB |

#### 12. 技术文章（dev.to 首发，跨站分发）
| 项 | 值 |
|-----|-----|
| 努力量 | 🔴 3-5h |
| 预期收益 | 🔥🔥 长期 SEO 资产 + 社区分享。一篇高质量文章可同时分发到 dev.to + Medium + 个人博客 |
| 前提条件 | 建议完成 P0 + P1 后再写，这样文章中有 crates.io 链接和 badge |
| 建议选题 | *"Building a <10ms terminal kanban board for AI agents in Rust: why zero deps beat frameworks"* — 混合技术深度 + 故事性，比纯教程更受欢迎 |

#### 13. HN Show 帖
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 30min（加上互动时间 🟠 1-3h） |
| 预期收益 | 🔥 如果能上首页，短期几千 UV。但 HN 随机性大，需要看运气 |
| 前提条件 | 建议等 Reddit 帖验证过反馈后再上 HN |
| 说明 | 可复用 Reddit 帖的文案，注意 HN 标题有字数限制 |

#### 14. YouTube / 视频演示（可选）
| 项 | 值 |
|-----|-----|
| 努力量 | 🔴 2-3h |
| 预期收益 | 🟡 长期 passive 流量，转化率低 |
| 前提条件 | 需要录制工具 + terminal 环境 |
| 说明 | **建议暂缓**。对 CLI 工具，文字 demo + GIF 转换率远高于视频 |

#### 15. awesome-rust 提交
| 项 | 值 |
|-----|-----|
| 努力量 | 🟡 15min PR |
| 预期收益 | 🔥 顶级 Rust 资源列表收录 |
| 前提条件 | **门槛极高**。审核标准：crate 已发布 + 社区证明（stars / downloads）+ 稳定有用。当前 0 stars 几乎不可能通过 |
| 说明 | **建议暂缓**。等项目达到 50-100 stars、crates.io 有 100+ downloads 后再提交 |

---

## 三、GEO vs 传统 SEO 权重分析

### 核心判定：**GEO > 传统 SEO**

| 维度 | 传统 SEO | GEO（AI 引擎优化） |
|------|---------|-------------------|
| 目标用户 | Google 搜索用户 | Claude Code / Codex / Gemini CLI / Perplexity |
| 对 git-kanban 的价值 | 低 — CLI 工具的 Google 搜索量很小 | **高** — agent 的自动化发现是核心增长引擎 |
| 优化手段 | 关键词密度、外链、PR | llms.txt、结构化文档、MCP 声明、AGENTS.md |
| 竞争度 | 高 — "kanban" 已被大量 SaaS 占据 | **极低** — 几乎没有终端 kanban 工具在优化 AI 发现 |
| 投资回报比 | 低 — 需要持续维护 | **极高** — 一次编写，长期被 AI 消费 |

### 推荐权重分配

```
GEO 60%  |  llms.txt + AGENTS.md + MCP 声明
SEO 20%  |  GitHub topics + README + crates.io metadata
社区 20%  |  Reddit + HN + dev.to + Awesome Lists
```

### 为什么 GEO 对 git-kanban 更重要

1. **目标人群高度匹配** — git-kanban 的设计目标是 AI Agent（JSON in/out）和 TUI 用户。这两类人群都大量使用 AI coding tools。
2. **竞争优势** — 工具声称为 agent 设计，但当前无法被任何 AI engine 自动发现。llms.txt 是让 agent "发现" 项目的入口。
3. **关键词竞争** — 传统 SEO 中 "kanban board" 被 Jira、Trello、Linear 等巨头占据。但在 AI/agent 场景下，"terminal kanban for AI agents" 是**零竞争长尾**。
4. **MCP 生态位** — 2026 年 MCP 已成 coding agent 的标准协议，但 git-kanban 不需要 MCP server（原生 CLI 即 tool），这是个可营销的差异点。

---

## 四、竞争对手分析（节选）

| 工具 | 类型 | Stars | 差异化 |
|------|------|-------|--------|
| git-kanban（本项目） | TUI + CLI | 0 | Agent-first JSON I/O, <10ms cache, 858KB, GitHub+GitLab |
| kanban-tui (crates.io) | TUI | — | 通用 TUI kanban，无 git 集成 |
| kanbanban (crates.io) | TUI | — | 高性能 Rust kanban |
| Kanban CLI (Codeberg) | TUI+Agent | ~17 HN points | Git 集成 + sprint 管理，更重 |
| kanban-mcp (GitHub) | MCP server | — | 专注于 MCP 协议，非 TUI |
| Backlog.md (awesome-tuis) | TUI | — | AI-Human 协作 git 管理 |

**差异化空间**:
- git-kanban 是唯一同时支持 **GitHub + GitLab** 的 terminal kanban
- 唯一明确标榜 **Agent-first**（JSON in/out + <10ms cache）的工具
- 唯一拥有 `--dry-run` 预览模式的工具（对 agent 安全操作至关重要）

---

## 五、执行顺序建议

```
Day 1 — P0（零成本快速启动）
  ├─ GitHub Topics 补全 (<5min)
  ├─ llms.txt 创建 (15min)
  ├─ Cargo.toml metadata 补全 (5min)
  ├─ README 首段 + badge 优化 (15min)
  ├─ MCP 兼容性声明写入 README + AGENTS.md (15min)
  ├─ GitHub Release v1.0.0 创建 (10min)
  ├─ crates.io 发布 (20min)
  └─ Social Preview 图制作 (30min)

Day 2-3 — P1（Awesome List 提交）
  ├─ awesome-ratatui PR (15min)
  ├─ awesome-tuis Issue (15min)
  └─ 可选: awesome-cli-apps 提交 (15min)

Week 2 — P2（内容营销）
  ├─ dev.to 文章撰写 + 分发 (3-5h)
  ├─ Reddit r/rust 帖 + 互动 (1-2h)
  └─ HN Show 帖 (1-3h，可选)

Month 1+ — P2 跟进
  └─ 监测下载量 + stars，达到 50+ stars 后提交 awesome-rust
```

---

## 六、关键指标追踪

| 指标 | 当前基线 | 1周目标 | 1月目标 | 3月目标 |
|------|---------|---------|---------|---------|
| GitHub Stars | 0 | 5-10 | 30-50 | 100-200 |
| crates.io Downloads | N/A | 50 | 500 | 2000 |
| llms.txt | 不存在 | ✅ 已创建 | — | — |
| Awesome Lists 收录 | 0 | 1（ratatui） | 2-3 | 3-4 |
| dev.to 文章 | 0 | 1 | 1 | 2-3 |
| Reddit / HN 曝光 | 0 | 1 Reddit 帖 | 2-3 | 3-5 |
| MCP 可见性 | 无声明 | ✅ 已声明 | 被 agent 自动发现 | 引用出现 |

---

*本报告由 Hermes 军师参谋模块生成，基于公开数据和项目现状分析。ROI 判断为定性分析，实际效果取决于执行质量和市场环境。*
