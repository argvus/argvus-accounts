# argvus-accounts

Local user account and avatar manager for the **Argvus** desktop.

`argvus-accounts` is the official source of account metadata used across the
Argvus ecosystem. It manages local Linux accounts — display names, avatars and
group membership — through standard system mechanisms (NSS, GECOS,
shadow-utils), without touching the Unix username and without introducing any
daemon.

```
argvus-accounts
      |
      v
account metadata / avatar (~/.face)
      |
      v
argvus-greeter  (read-only consumer)
```

The greeter has **no administrative responsibility** over accounts: it only
reads what `argvus-accounts` wrote. Users manage their own profile with this
tool; anything affecting other users or groups requires administrator
privileges.

## Features

- List local users (`list`) with optional system accounts (`--all`).
- Inspect an account (`show`, `self`): display name, home, shell, groups, avatar.
- Change the display name (`name`, `self name`) stored in the standard GECOS
  full-name field — the Unix username is never modified.
- Set/remove the user avatar (`avatar`, `self avatar`): validated, normalized
  to a square 256x256 PNG and deployed atomically to `~/.face`.
- Query group membership (`groups`) and mutate it administratively
  (`groups --add/--remove`) via `gpasswd`.
- Reusable library crate (`argvus-accounts-core`) ready for a future Argvus
  GUI (`argvus-accounts-gtk` / `argvus-settings`).
- Authorization layer (`AuthorizationProvider`) designed for a future polkit
  integration; today it implements classic Unix semantics (root = admin).

## Architecture

```
crates/
├── argvus-accounts/          # CLI binary (clap parsing, dispatch, output)
└── argvus-accounts-core/     # reusable library
    └── src/
        ├── lib.rs            # public API / docs
        ├── error.rs          # typed errors (thiserror)
        ├── validation.rs     # strict username/group/display-name validation
        ├── passwd.rs         # NSS user database access (getpwnam/getpwent)
        ├── groups.rs         # NSS group lookups + getgrouplist + gpasswd
        ├── metadata.rs       # GECOS display name + external tool runners
        ├── avatar.rs         # image validation, normalization, atomic deploy
        ├── permissions.rs    # Action/AuthorizationProvider abstraction
        └── account.rs        # AccountManager facade + PrivilegeOps trait
```

Design rules:

- Reads use NSS APIs, never hand-parsed `/etc/passwd`/`/etc/group`.
- Writes never edit account files directly. Display names go through
  `usermod --comment` (admin) or `chfn --full-name` (self); group changes go
  through `gpasswd -a/-d`. External tools are spawned with argument vectors —
  there is no shell anywhere in the codebase.
- All user input is strictly validated before it can reach NSS, the
  filesystem or an external tool.
- Avatar updates are atomic (tempfile -> fsync -> chmod/chown -> rename), so
  an interrupted update never leaves the user without an avatar.
- The original source image is copied/normalized; it is never moved or
  modified.

## Installation

### Arch Linux (from source)

```sh
git clone https://github.com/argvus/argvus-accounts
cd argvus-accounts
make build
sudo make install
```

Or build the package:

```sh
cd packaging
makepkg -si
```

### From source (any distribution)

```sh
cargo install --path crates/argvus-accounts
```

Runtime dependencies: `shadow` (usermod/gpasswd) and `util-linux` (chfn).
Both ship with every default Arch installation. Reading avatars requires no
extra dependencies.

## Usage

```text
$ argvus-accounts list
USER             NAME
ghost            Ghost
william          William Canin

$ argvus-accounts show ghost
Username: ghost
Name: Ghost
UID: 1001
GID: 1001
Home: /home/ghost
Shell: /bin/bash
Groups: audio, video, wheel
Avatar: /home/ghost/.face

$ argvus-accounts self avatar ~/Pictures/avatar.png
Avatar updated successfully.

$ argvus-accounts self avatar --remove
Avatar removed successfully.

$ argvus-accounts self name "William Canin"
Display name updated successfully.

$ sudo argvus-accounts name ghost "Ghost"
Display name updated successfully.

$ sudo argvus-accounts groups ghost --add wheel
Added 'ghost' to group 'wheel'.

$ sudo argvus-accounts groups ghost --remove wheel
Removed 'ghost' from group 'wheel'.
```

### Command reference

| Command | Description | Privileges |
|---|---|---|
| `list [--all]` | List human users (all accounts with `--all`) | none |
| `show USER` | Account details | none |
| `name USER NAME` | Set display name of another user | root |
| `avatar USER IMAGE` / `avatar USER --remove` | Manage another user's avatar | root |
| `groups USER` | List groups of a user | none |
| `groups USER --add G [--add G2]` | Add memberships | root |
| `groups USER --remove G [--remove G2]` | Remove memberships | root |
| `self` / `self show` | Show own account | none |
| `self name NAME` | Change own display name | own password via PAM (chfn) |
| `self avatar IMAGE` / `self avatar --remove` | Manage own avatar | none |
| `self groups` | List own groups | none |

Use `--verbose` for diagnostics on any subcommand.

## Permissions model

| Operation | Regular user | Admin/root |
|---|---|---|
| Read account info | yes | yes |
| Change own display name | yes (via chfn/PAM) | yes |
| Change own avatar | yes | yes |
| Change another user's data | **no** | yes |
| Group membership changes | **no** (even own) | yes |

Privileges are decided by the pluggable [`AuthorizationProvider`] layer. The
default implementation follows Unix semantics (`euid == 0` is an
administrator); unprivileged callers may act only on their own profile. The
interface is ready for a `PolkitAuthorizationProvider` mapping:

- `ModifyOwnAccount`   -> `com.argvus.accounts.change-own-data` (`allow_active=yes`)
- `ModifyOtherAccount` -> `com.argvus.accounts.administer`     (`auth_admin`)
- `AdministerGroups`   -> `com.argvus.accounts.administer`     (`auth_admin`)

## Avatar location and format contract

Avatars live at:

    $HOME/.face

This is the long-standing Unix/freedesktop convention already consumed by
LightDM, SDDM and most desktop environments, so Argvus avatars work with
other display managers too. The alternative freedesktop mechanism,
AccountsService, was considered but requires its D-Bus daemon for writes; it
remains a possible future mirror target, not a dependency.

Contract guaranteed by `argvus-accounts`:

- regular file (never a symlink);
- PNG encoded;
- exactly 256x256 pixels (center-cropped, aspect preserved);
- mode `0644`;
- owned by the user (uid/gid of the account);
- accepted input formats: PNG, JPEG, WebP — detected from file contents, not
  extensions; maximum source size 20 MiB; decompression limits guard against
  image bombs.

For the greeter to read `$HOME/.face`, home directories must remain traversable
(the Arch default `0755`). Hardened setups that restrict homes should either
relax traversal for the greeter user or adopt a future AccountsService mirror.

## Security notes

- Usernames/group names are validated against `[a-z_][a-z0-9_-]{0,31}`:
  path traversal, option injection and NSS field injection are impossible by
  construction.
- No shell is ever executed; external tools receive argument vectors built
  after validation, with end-of-options separators (`--`).
- The avatar destination path is derived solely from NSS data plus a constant
  filename; the caller cannot choose arbitrary destinations.
- Deployment verifies home ownership before writing; temporary files are
  created inside the target home, fsynced, permissioned, chowned and renamed
  atomically (symlinks at `.face` are replaced, not followed).
- A directory named `.face` is refused rather than removed recursively.
- Errors are typed and actionable; internals are not leaked in messages.

## Integration with Argvus Greeter

`argvus-greeter` must remain a pure consumer. When greetd starts the greeter
and the user selects `ghost`, the greeter discovers and loads the avatar with
two calls from this library (or plain filesystem reads):

```rust
use argvus_accounts_core::{get_user_by_name, avatar};

let user = get_user_by_name("ghost")?.expect("user exists");
if let Some(path) = avatar::find_avatar(&user.home) {
    let bytes = std::fs::read(path)?; // always a regular 256x256 PNG
    // render...
}
```

Rules enforced by the architecture:

- no avatar switching logic inside the greeter;
- no authentication/write paths in the greeter;
- users change their avatar *before* login, through `argvus-accounts` (CLI)
  or a future Argvus GUI built on `argvus-accounts-core`.

## Development

```sh
cargo build                 # debug build
cargo build --release       # optimized build (LTO, stripped)
cargo test                  # unit + integration tests (hermetic, no root)
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt && cargo fmt --check
make check                  # clippy + tests
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for layout details and
[CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

Tests never modify real accounts or real home directories: privileged
operations run against mock backends and avatar deployment targets temporary
directories.

## Packaging

`packaging/arch/PKGBUILD` builds the Arch package. It installs only
`/usr/bin/argvus-accounts` plus documentation/license — no users are created,
no dotfiles touched, no services enabled during installation.

## Roadmap

- polkit-backed authorization provider (no more required `sudo`);
- optional AccountsService mirror for restricted-home setups;
- `argvus-accounts-gtk` GUI on top of the core crate;
- shell completions.

## License

GPL-3.0-only — see [LICENSE](LICENSE).

[`AuthorizationProvider`]: crates/argvus-accounts-core/src/permissions.rs
