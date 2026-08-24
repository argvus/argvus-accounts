# Contributing to argvus-accounts

Thanks for helping build Argvus!

## Ground rules

1. **Safety first**: this tool touches system accounts. Any change must keep
   the guarantees documented in the README (strict validation, no shell
   execution, atomic avatar updates, pluggable authorization).
2. **Library-first**: business logic belongs in `argvus-accounts-core`. The
   CLI is a thin layer on top and future GUIs will reuse the same core.
3. **No new conventions without discussion**: prefer freedesktop/Linux
   standards over project-specific mechanisms.

## Workflow

```sh
fork & clone
cargo fmt
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
```

Commit style (Conventional Commits):

```
feat(core): short imperative summary
fix(cli): ...
docs(readme): ...
test(core): ...
build(arch): ...
chore: ...
```

Keep each commit focused on one responsibility. Pull requests must pass CI
(formatting, build, tests, clippy) and include tests for any behavior change.

## Reporting issues

Include distribution/version, how you ran the command, the full output with
`--verbose`, and what you expected to happen. Never paste password prompts or
private data.
