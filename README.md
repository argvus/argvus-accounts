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
- Change passwords (`passwd`): self-service with current-password proof via
  the system's setuid `unix_chkpwd` helper, or administrator reset.
- Automatic privilege elevation through polkit (`pkexec`) — administrative
  operations work without `sudo`; a polkit agent handles authorization.
- Reusable library crate (`argvus-accounts-core`) ready for a future Argvus
  GUI (`argvus-accounts-gtk` / `argvus-settings`).
- Authorization layer (`AuthorizationProvider`) implementing classic Unix
  semantics (root = admin) with pluggable providers.

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
  `usermod --comment`; group changes go through `gpasswd -a/-d`. External
  tools are spawned with argument vectors — there is no shell anywhere in the
  codebase.
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

Runtime dependencies: `shadow` (usermod/gpasswd/chpasswd), `util-linux`,
`linux-pam` (setuid `unix_chkpwd`, used to verify the current password) and
`polkit` (automatic elevation). All ship with every default Arch installation.

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

$ argvus-accounts name ghost "Ghost"
argvus-accounts: requesting administrator privileges via polkit…
Display name updated successfully.

$ argvus-accounts groups ghost --add wheel
argvus-accounts: requesting administrator privileges via polkit…
Added 'ghost' to group 'wheel'.

$ argvus-accounts passwd william current-pass new-secret-1 new-secret-1
Password updated successfully for 'william'.
```

No `sudo` prefix is required: whenever an operation needs administrator
privileges, the CLI re-executes itself through polkit (`pkexec`) and your
desktop agent asks for authorization. Prefer a terminal prompt? Keep using
`sudo` — both work.

### Changing passwords

```sh
# Your own password: the current one is required as proof.
argvus-accounts passwd $USER <OLD_PASSWD> <NEW_PASSWD> <CONFIRM_PASSWD>

# Administrator reset of another user (current password not needed).
sudo argvus-accounts passwd ghost ignored new-secret-1 new-secret-1
```

Rules and guarantees:

- confirmation must match exactly; only transport constraints are enforced
  locally — the value cannot be empty and cannot contain line breaks or NUL
  bytes (they cannot cross the `chpasswd`/`unix_chkpwd` pipes). There is **no
  complexity policy** here: minimum lengths, character classes and strength
  checks belong to frontends or the PAM stack;
- self-service changes verify the current password through the system's
  setuid helper (`unix_chkpwd`, the same mechanism `pam_unix` uses) *before*
  requesting elevation, so a typo never triggers an administrator dialog;
- secrets travel exclusively through stdin pipes to `unix_chkpwd` /
  `chpasswd` — never through argument vectors between tools;
- administrators may reset other users' passwords; in that case the current
  password argument is ignored.

### Command reference

| Command | Description | Privileges |
|---|---|---|
| `list [--all]` | List human users (all accounts with `--all`) | none |
| `show USER` | Account details | none |
| `name USER NAME` | Set display name of another user | auto-elevates via polkit |
| `avatar USER IMAGE` / `avatar USER --remove` | Manage another user's avatar | auto-elevates via polkit |
| `groups USER` | List groups of a user | none |
| `groups USER --add G [--add G2]` | Add memberships | auto-elevates via polkit |
| `groups USER --remove G [--remove G2]` | Remove memberships | auto-elevates via polkit |
| `passwd USER OLD NEW CONFIRM` | Change password (own: proof required; others: admin reset) | self: none to verify; write auto-elevates via polkit |
| `self` / `self show` | Show own account | none |
| `self name NAME` | Change own display name | auto-elevates via polkit |
| `self avatar IMAGE` / `self avatar --remove` | Manage own avatar | none |
| `self groups` | List own groups | none |

Use `--verbose` for diagnostics on any subcommand.

## Permissions model

| Operation | Regular user | Admin/root |
|---|---|---|
| Read account info | yes | yes |
| Change own display name | yes (auto-elevates via polkit) | yes |
| Change own avatar | yes | yes |
| Change own password | yes (current password proof required) | yes |
| Change another user's data | **no** (auto-elevates via polkit) | yes |
| Group membership changes | **no** (auto-elevates via polkit) | yes |
| Reset another user's password | **no** (auto-elevates via polkit) | yes |

Privileges are decided by the pluggable [`AuthorizationProvider`] layer. The
default implementation follows Unix semantics (`euid == 0` is an
administrator); unprivileged callers may act only on their own profile. The
interface is ready for a `PolkitAuthorizationProvider` mapping:

- `ModifyOwnAccount`   -> `com.argvus.accounts.change-own-data` (`allow_active=yes`)
- `ModifyOtherAccount` -> `com.argvus.accounts.administer`     (`auth_admin`)
- `AdministerGroups`   -> `com.argvus.accounts.administer`     (`auth_admin`)
- `ChangePassword`     -> self-service with proof; admin reset otherwise

### Automatic elevation (polkit)

When an operation needs root and the process is not privileged, the CLI
re-executes itself via `pkexec` with the same arguments. The elevated child is
tagged with an internal environment guard, so authorization happens exactly
once and a denial inside the child surfaces as a normal error instead of
another prompt. pkexec missing (no polkit installed) produces an actionable
error suggesting `sudo`.

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
- Password secrets never appear in argument vectors between tools: they are
  streamed to `unix_chkpwd` / `chpasswd` through pipes. Note the inherent CLI
  limitation that positional arguments (including passwords) are briefly
  visible in your shell history and the system process table; prefer a
  private terminal and consider rotating if that matters in your threat
  model. Interactive prompt mode is planned.
- Elevation uses polkit with a single-attempt guard; no password is ever
  read or cached by `argvus-accounts` itself.
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

- dedicated polkit action files (`com.argvus.accounts.*`) with fine-grained
  rules (today: generic pkexec elevation);
- interactive password prompt mode (secrets never on argv);
- optional AccountsService mirror for restricted-home setups;
- `argvus-accounts-gtk` GUI on top of the core crate;
- shell completions.

## License

GPL-3.0-only — see [LICENSE](LICENSE).

[`AuthorizationProvider`]: crates/argvus-accounts-core/src/permissions.rs
