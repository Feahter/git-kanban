# Changelog

## [1.0.1] — 2026-11-09

Agent-contract fixes. No TUI changes.

### Fixed — the documented commands did not run
- Every top-level flag is now `global = true`. `git-kanban close 1 --repo o/n` (the form used in
  every doc block) exited 2 with `unexpected argument '--repo'`; so did `--dry-run` after a
  subcommand. All documented invocations now parse.

### Fixed — a write could silently hit the wrong repository
- The write path always used the config repo, so `--repos a --repos b close 5` closed #5 in the
  *config* repo and reported `ok:true`. Now: `--repo` > single `--repos` > config; more than one
  `--repos` with a write is a usage error (exit 2).

### Fixed — silent degradation
- `--fields <unknown>` emitted `issues: [{}]` with exit 0 → now exit 2 listing valid fields.
- `--column <unknown>` returned every issue unfiltered with exit 0 → now exit 2 listing valid ids.
- `close 12,abc` closed only #12 and reported `ok:true` → non-numeric tokens are now exit 2.

### Fixed — `--dry-run` was not a safe preview
- It required `gh auth status` first, so it failed without a token. Auth is now skipped in dry-run.
- `open` launched a browser and `labels` hit the network under `--dry-run`; both now preview.
- Receipts now carry the resolved `repo` and `backend` (they previously carried neither, so a
  preview could not confirm the target). `create` receipts also echo `--body`.

### Fixed — `--json` was ignored by two modes
- `--json --refresh` printed `Cached N issues from …` as text → now emits a JSON receipt.
- `--json --refresh` on multiple repos likewise.

### Docs
- Corrected stale numbers (858 KB → 1.05 MB measured; "4 crates" → 5 direct deps), removed the
  documented-but-nonexistent `--quiet`, fixed the cache filename, documented exit code 2 and the
  four contract invariants.
- `tests/cli_test.rs`: 6 network-free contract tests added (flag position, ambiguous write target,
  malformed number, unknown field/column, dry-run receipt contents).

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
