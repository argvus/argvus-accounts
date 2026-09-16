# Git Hooks

Enable the repository hooks once with:

```sh
git config core.hooksPath hooks
```

The pre-commit hook runs `make check`; `commit-msg` validates Conventional
Commit messages.
