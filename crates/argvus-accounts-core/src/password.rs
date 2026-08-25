//! Password management primitives.
//!
//! Two concerns live here:
//!
//! - verifying the caller's *current* password without linking against PAM:
//!   the shadow suite ships the setuid helper `unix_chkpwd` (the very helper
//!   `pam_unix` uses), which verifies a password for the invoking uid and
//!   refuses to check other users' passwords. This keeps self-service
//!   password changes honest without new native dependencies;
//! - setting a password as root via `chpasswd`, feeding the secret through a
//!   pipe. Passwords never travel through argument vectors or shell strings.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// Minimum accepted length for a newly chosen password.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Maximum length accepted from stdin by `unix_chkpwd`/`chpasswd` protocols.
const MAX_PASSWORD_LEN: usize = 127;

/// Well-known locations of the setuid verification helper.
const CHKPWD_CANDIDATES: [&str; 3] = [
    "/usr/bin/unix_chkpwd",
    "/usr/sbin/unix_chkpwd",
    "/sbin/unix_chkpwd",
];

/// Validates a password chosen by a user.
///
/// The rules keep the value compatible with both backends used here: no
/// control characters (they would break the stdin protocols) and a minimum
/// length aligned with common PAM policies.
pub fn validate_new_password(password: &str) -> Result<()> {
    if password.is_empty() {
        return Err(Error::InvalidPassword(
            "password cannot be empty".to_string(),
        ));
    }
    if password.len() > MAX_PASSWORD_LEN {
        return Err(Error::InvalidPassword(format!(
            "password is too long (maximum is {MAX_PASSWORD_LEN} bytes)"
        )));
    }
    if password.len() < MIN_PASSWORD_LEN {
        return Err(Error::InvalidPassword(format!(
            "password is too short (minimum is {MIN_PASSWORD_LEN} characters)"
        )));
    }
    if password
        .chars()
        .any(|c| c == '\n' || c == '\r' || c.is_control())
    {
        return Err(Error::InvalidPassword(
            "password must not contain control characters or line breaks".to_string(),
        ));
    }
    Ok(())
}

/// Returns the path of the setuid verification helper, when present.
#[must_use]
pub fn chkpwd_helper_path() -> Option<std::path::PathBuf> {
    CHKPWD_CANDIDATES
        .iter()
        .map(std::path::PathBuf::from)
        .find(|path| path.exists())
}

/// Verifies `password` for `username` using the system's setuid helper.
///
/// Mirrors what `pam_unix` does for authentication. The helper enforces that
/// unprivileged callers may only verify their own account, so this function is
/// meaningful for self-service flows; administrators skip verification
/// entirely when resetting another user's password.
///
/// Returns `Ok(false)` for wrong passwords, unknown users and locked
/// accounts; only infrastructure failures (helper missing, spawn error)
/// surface as [`Error`].
pub fn verify_own_password(username: &str, password: &str) -> Result<bool> {
    let Some(helper) = chkpwd_helper_path() else {
        return Err(Error::system(
            "password verification helper unix_chkpwd was not found; \
             is the 'shadow' package installed?",
        ));
    };

    let mut child = Command::new(&helper)
        .args([username, "chkpasswd"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| Error::system(format!("failed to launch {}: {err}", helper.display())))?;

    if let Some(mut stdin) = child.stdin.take() {
        // The helper reads at most one line; a trailing newline is expected.
        stdin
            .write_all(password.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .ok();
        drop(stdin);
    }

    match child.wait() {
        Ok(status) => Ok(status.success()),
        Err(err) => Err(Error::system(format!(
            "failed to run {}: {err}",
            helper.display()
        ))),
    }
}

/// Sets `username`'s password to `new` using `chpasswd` (root context).
///
/// The secret travels through an in-memory pipe, never through argv.
pub fn set_password_admin(username: &str, new: &str) -> Result<()> {
    validate_new_password(new)?;

    let mut child = Command::new("chpasswd")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| Error::system(format!("failed to launch chpasswd: {err}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(format!("{username}:{new}\n").as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|err| Error::system(format!("failed to send payload to chpasswd: {err}")))?;
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .map_err(|err| Error::system(format!("failed to run chpasswd: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::system(format!(
            "chpasswd rejected the new password{}",
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", stderr.trim())
            }
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_empty_and_control_passwords() {
        assert!(validate_new_password("").is_err());
        assert!(validate_new_password("a1b2c3d").is_err());
        assert!(validate_new_password("a1b2c3d4\n").is_err());
        assert!(validate_new_password("a1b2c3d4\r").is_err());
        assert!(validate_new_password("a1b2c3\u{7f}").is_err());
        assert!(validate_new_password("a1b2c3d4").is_ok());
        let long = "x".repeat(MAX_PASSWORD_LEN + 1);
        assert!(validate_new_password(&long).is_err());
    }

    #[test]
    fn colons_and_spaces_are_valid_password_bytes() {
        assert!(validate_new_password("pa ss:word1").is_ok());
    }

    #[test]
    fn wrong_password_is_reported_without_error_when_helper_exists() {
        let Some(helper) = chkpwd_helper_path() else {
            return; // non-shadow environment: nothing to verify against
        };
        let me = crate::passwd::current_user().expect("current user resolves");
        let ok =
            verify_own_password(&me.username, "definitely-not-my-password").expect("helper runs");
        assert!(!ok, "wrong password must not verify ({})", helper.display());
    }

    #[test]
    fn unknown_user_verifies_false_instead_of_erroring() {
        if chkpwd_helper_path().is_none() {
            return;
        }
        let ok = verify_own_password("argvus-no-such-user-42", "whatever-1").expect("helper runs");
        assert!(!ok);
    }
}
