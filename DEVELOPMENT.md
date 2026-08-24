# Development Guide

## Layout

```
crates/
├── argvus-accounts/          CLI binary
│   ├── src/main.rs           entry point (parse -> dispatch -> errors)
│   ├── src/cli.rs            clap definitions
│   ├── src/commands.rs       command implementations / output
│   └── tests/cli.rs          read-only end-to-end tests (assert_cmd)
└── argvus-accounts-core/     reusable library
    ├── src/error.rs          typed errors (thiserror)
    ├── src/validation.rs     username/group/display-name rules
    ├── src/passwd.rs         NSS user access (nix + getpwent FFI)
    ├── src/groups.rs         NSS group access + gpasswd builders
    ├── src/metadata.rs       GECOS display name + external tool runner
    ├── src/avatar.rs         validation/normalization/atomic deploy
    ├── src/permissions.rs    Action + AuthorizationProvider trait
    └── src/account.rs        AccountManager facade + PrivilegeOps trait
packaging/PKGBUILD            Arch Linux package
```

## Commands

```sh
cargo build --release --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo fmt && cargo fmt --check
make check      # clippy + tests
make install    # release build + install to PREFIX
```

## Conventions

- All logic lives in `argvus-accounts-core`; the CLI only parses, dispatches
  and formats. Future GUIs should depend on the core crate only.
- Never edit `/etc/passwd` or `/etc/group` directly. Reads go through NSS,
  writes through `usermod`/`chfn`/`gpasswd` spawned with argument vectors.
- Validate every identifier (`validation.rs`) before it reaches NSS, the
  filesystem or a subprocess.
- Keep the authorization decision inside an `AuthorizationProvider`
  implementation — never inline privilege checks.
- Avatar changes must stay atomic: validate -> temp file -> fsync ->
  chmod/chown -> rename.

## Testing rules

Tests must be hermetic:

- privileged operations are recorded by mock `PrivilegeOps` backends;
- avatar deployment targets temporary homes via
  `AccountManager::with_home_override`;
- CLI tests are read-only (`list`, `show`, invalid inputs);
- no test may write to the running user's real `$HOME` or require root.

## Release flow

1. Bump versions in both crate manifests (workspace version).
2. Update `pkgver` handling: the tag workflow rewrites `packaging/PKGBUILD`
   automatically from the pushed `vX.Y.Z` tag.
3. Tag the release (`git tag vX.Y.Z`); CI builds and publishes the Arch
   package to `argvus/packages`.
