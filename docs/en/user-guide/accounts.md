---
title: User Accounts
description: Manage user profiles, display names, avatars, and groups.
---

`argvus-accounts` is the official account manager for the ARGVUS desktop. It manages local user profiles without requiring administrative privileges for most operations — display names, avatars, and group memberships are handled through your desktop's native authorization system.

## List users

```sh
argvus-accounts list              # Show human users
argvus-accounts list --all        # Include system accounts
```

## View account information

```sh
argvus-accounts show <username>   # Display account details
argvus-accounts self              # Show your own account
argvus-accounts self show         # Equivalent to 'self'
```

Output includes username, UID, GID, home directory, shell, group memberships, and avatar location.

## Manage display name

Change how your name appears in the greeter and desktop:

```sh
argvus-accounts self name "Your Display Name"    # Change your own name
argvus-accounts name <user> "Display Name"       # Admin: change another user's name
```

Display names are stored in the standard GECOS field (the Unix `comment` field). The username itself is never modified.

## Manage avatars

Avatars appear in the greeter and user profile areas. The file must be an image (PNG, JPEG, or WebP). It is automatically resized to 256×256 pixels and converted to PNG.

```sh
# Set or replace your avatar
argvus-accounts self avatar ~/Pictures/profile-pic.png

# Remove your avatar
argvus-accounts self avatar --remove

# Admin: set another user's avatar
argvus-accounts avatar <user> ~/Pictures/avatar.png
argvus-accounts avatar <user> --remove
```

Avatars are stored at `~/.face` following the standard freedesktop convention. This location works with LightDM, SDDM, and other display managers as well.

**Avatar requirements**:
- Supported formats: PNG, JPEG, WebP
- Maximum input size: 20 MiB
- Output: always 256×256 PNG with mode 0644

## Manage group membership

View and modify which groups a user belongs to. This may require administrator authorization.

```sh
# View group membership
argvus-accounts groups <user>
argvus-accounts self groups

# Add to a group (requires authorization)
argvus-accounts groups <user> --add wheel
argvus-accounts groups <user> --add audio --add video

# Remove from a group (requires authorization)
argvus-accounts groups <user> --remove wheel
```

Common groups include `wheel` (administrator access), `audio` (sound access), `video` (graphics acceleration), and `input` (keyboard/mouse input handling).

## Change password

Change your password or (if you are an administrator) reset another user's password.

```sh
# Change your own password (current password required as proof)
argvus-accounts passwd <user> <old-password> <new-password> <confirm-password>

# Administrator password reset (current password is ignored)
sudo argvus-accounts passwd <user> ignored <new-password> <confirm-password>
```

The confirmation password must match the new password exactly. While typed on the command line here, passwords should ideally be entered interactively (a future improvement). Administrators can reset passwords without knowing the current one; regular users must prove they remember their current password before changing it.

## Authorization

Most `argvus-accounts` commands work without administrator privileges:

| Operation | Regular user | Administrator |
| --- | --- | --- |
| List users | ✓ | ✓ |
| View account info | ✓ | ✓ |
| Change own display name | ✓ | ✓ |
| Change own avatar | ✓ | ✓ |
| Change own password | ✓ (with proof) | ✓ |
| Modify other users | — | ✓ |
| Change groups | — | ✓ |
| Reset other passwords | — | ✓ |

When a regular user tries an administrative operation (like changing another user's name), your desktop's authorization agent prompts for permission. There is no need to use `sudo` — `argvus-accounts` automatically requests elevation through the system's standard PolicyKit mechanism.

## Verbose mode

For diagnostics, add `--verbose` to any command:

```sh
argvus-accounts --verbose list
argvus-accounts --verbose self avatar ~/Pictures/avatar.png
```

This prints detailed information about what the command is doing.

## Examples

**Set up your profile after login**:

```sh
# Set your display name
argvus-accounts self name "William Canin"

# Upload an avatar
argvus-accounts self avatar ~/Pictures/profile.png
```

**Manage group membership**:

```sh
# Check your current groups
argvus-accounts self groups

# Request to join a group (if your administrator allows)
argvus-accounts groups $USER --add wheel
```

**System administrator tasks**:

```sh
# List all users and system accounts
argvus-accounts list --all

# Check a user's groups and avatar
argvus-accounts show ghost
argvus-accounts groups ghost

# Update a user's profile
argvus-accounts avatar ghost ~/Desktop/ghost-profile.png
argvus-accounts name ghost "Ghost Account"
```
