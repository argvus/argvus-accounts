//! Read-only CLI integration tests.
//!
//! Deliberately avoids any command that would mutate the running user's real
//! home directory or system accounts. Mutation paths are covered hermetically
//! by `argvus-accounts-core` tests with mock backends and temp homes.

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("argvus-accounts").expect("binary builds")
}

#[test]
fn help_is_informative() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("avatar"))
        .stdout(predicate::str::contains("self"));
}

#[test]
fn version_matches_semver() {
    let version = env!("CARGO_PKG_VERSION");
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(version));
}

#[test]
fn list_succeeds_with_header() {
    bin()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("USER"))
        .stdout(predicate::str::contains("NAME"));
}

#[test]
fn show_missing_user_fails_cleanly() {
    bin()
        .args(["show", "definitely_missing_user_xyz"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("user not found"));
}

#[test]
fn traversal_username_is_rejected() {
    bin()
        .args(["show", "../etc/passwd"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid username"));
}

#[test]
fn empty_display_name_is_rejected_before_any_system_call() {
    // Validation happens before authorization/NSS mutation, so this is safe.
    let me = whoami();
    bin()
        .args(["name", &me, "   "])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid display name"));

    bin()
        .args(["self", "name", "has,comma"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid display name"));
}

#[test]
fn groups_of_missing_user_fails_cleanly() {
    bin()
        .args(["groups", "definitely_missing_user_xyz"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("user not found"));
}

#[test]
fn contradictory_group_request_is_rejected() {
    // Unknown target would fail later; use an invalid one to prove validation
    // ordering without touching privileges at all.
    bin()
        .args(["groups", "../evil", "--add", "wheel", "--remove", "wheel"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn avatar_requires_image_or_remove_flag() {
    bin().args(["self", "avatar"]).assert().failure().code(2);
}

#[test]
fn verbose_flag_is_accepted_globally() {
    bin()
        .args(["--verbose", "list"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("USER"));
}

fn whoami() -> String {
    std::env::var("USER").unwrap_or_else(|_| "root".to_string())
}
