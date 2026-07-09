# Contributing

## Scope

This is a focused UI tool — small scope, high polish, zero unnecessary dependencies.

### The project will accept

- Bug fixes and edge-case patches
- Performance improvements
- Agent workflow enhancements (CLI subcommands, JSON flags)
- Documentation improvements

### The project will likely reject

- New backend support (GitLab is enough; no Jira/Linear/Notion)
- Embedded database or async runtime
- Webhook servers or daemon processes
- Large dependency additions

**When in doubt, open an issue first.**

## Style

- Run `cargo fmt` before committing
- Keep tests fast — no network calls in unit tests
- All CLI subcommands must work without TUI
- All TUI operations must have CLI equivalents

## PR Process

1. One feature or fix per PR
2. Update tests
3. Update `README.md` if the CLI surface changes (new flag/subcommand)
4. Reference the issue number in the PR body
