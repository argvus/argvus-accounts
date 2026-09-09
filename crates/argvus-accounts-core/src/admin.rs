//! Structured administration API. Secrets travel over stdin, never argv.
use std::process::Command;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{AccountManager, Error, Result, groups, metadata, passwd, password, validation};

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
enum Request {
    Snapshot,
    Create {
        user: String,
        name: String,
        shell: String,
        groups: Vec<String>,
    },
    Edit {
        user: String,
        name: String,
        shell: String,
        primary_group: String,
        groups: Vec<String>,
    },
    Password {
        user: String,
        old: String,
        new: String,
        confirm: String,
    },
    Lock {
        user: String,
        locked: bool,
    },
    ExpirePassword {
        user: String,
    },
    Delete {
        user: String,
    },
    CreateGroup {
        group: String,
    },
    Avatar {
        user: String,
        path: String,
    },
}

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidOperation(message.into())
}

fn actor_uid() -> Result<u32> {
    if passwd::effective_uid() == 0
        && let Ok(uid) = std::env::var("PKEXEC_UID")
    {
        return uid.parse().map_err(|_| invalid("invalid polkit caller"));
    }
    Ok(passwd::effective_uid())
}

fn shells() -> Result<Vec<String>> {
    let contents =
        std::fs::read_to_string("/etc/shells").map_err(|err| Error::system(err.to_string()))?;
    Ok(contents
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('/'))
        .map(str::to_owned)
        .collect())
}

fn validate_shell(shell: &str) -> Result<()> {
    if !shells()?.iter().any(|candidate| candidate == shell) {
        return Err(invalid("select a login shell listed in /etc/shells"));
    }
    Ok(())
}

fn validate_groups(names: &[String]) -> Result<()> {
    for name in names {
        validation::validate_group_name(name)?;
        if groups::get_group_by_name(name)?.is_none() {
            return Err(Error::GroupNotFound(name.clone()));
        }
    }
    Ok(())
}

fn target(user: &str) -> Result<passwd::UserInfo> {
    validation::validate_username(user)?;
    passwd::get_user_by_name(user)?.ok_or_else(|| Error::UserNotFound(user.into()))
}

fn protect_account(user: &str) -> Result<()> {
    let info = target(user)?;
    if info.uid < 1000 || info.uid == 65534 || info.uid == actor_uid()? {
        return Err(invalid(
            "cannot delete or lock a system account or your own account",
        ));
    }
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    metadata::run_command(Command::new(program).args(args), "account operation failed")
}

/// Executes a bounded JSON request. Only snapshots are available without root.
pub fn execute(input: &str) -> Result<Value> {
    if input.len() > 65536 {
        return Err(invalid("request too large"));
    }
    let request = serde_json::from_str(input).map_err(|_| invalid("invalid account request"))?;
    execute_request(request)
}

fn execute_request(request: Request) -> Result<Value> {
    if matches!(request, Request::Snapshot) {
        let manager = AccountManager::new();
        let mut users = Vec::new();
        for info in manager.list_users(true)? {
            let memberships = groups::groups_of_user(&info.username, info.gid)?;
            users.push(json!({
                "user": info.username, "name": info.display_name().unwrap_or(""),
                "uid": info.uid, "gid": info.gid, "home": info.home, "shell": info.shell,
                "primary_group": memberships.iter().find(|group| group.gid == info.gid).map(|group| group.name.clone()).unwrap_or_default(),
                "groups": memberships.iter().filter(|group| group.gid != info.gid).map(|group| group.name.clone()).collect::<Vec<_>>()
            }));
        }
        return Ok(
            json!({"users": users, "groups": groups::list_groups()?.into_iter().map(|group| group.name).collect::<Vec<_>>(), "shells": shells()?, "actor_uid": actor_uid()?}),
        );
    }
    if passwd::effective_uid() != 0 {
        return Err(Error::PermissionDenied(
            "administration requires pkexec argvus-accounts manage".into(),
        ));
    }
    apply_request(request, &mut run)
}

fn apply_request(
    request: Request,
    run: &mut impl FnMut(&str, &[&str]) -> Result<()>,
) -> Result<Value> {
    match request {
        Request::Snapshot => unreachable!(),
        Request::Create {
            user,
            name,
            shell,
            groups,
        } => {
            validation::validate_username(&user)?;
            let name = metadata::validate_display_name(&name)?;
            validate_shell(&shell)?;
            validate_groups(&groups)?;
            if passwd::get_user_by_name(&user)?.is_some() {
                return Err(invalid("user already exists"));
            }
            // A new account remains password-locked until a separate password operation succeeds.
            run(
                "/usr/bin/useradd",
                &[
                    "--create-home",
                    "--user-group",
                    "--comment",
                    &name,
                    "--shell",
                    &shell,
                    "--groups",
                    &groups.join(","),
                    "--",
                    &user,
                ],
            )?;
        }
        Request::Edit {
            user,
            name,
            shell,
            primary_group,
            groups,
        } => {
            target(&user)?;
            let name = metadata::validate_display_name(&name)?;
            validate_shell(&shell)?;
            validate_groups(&groups)?;
            validate_groups(std::slice::from_ref(&primary_group))?;
            if target(&user)?.uid == actor_uid()? {
                let current = groups::groups_of_user(&user, target(&user)?.gid)?;
                if current.iter().any(|group| group.name == "wheel")
                    && primary_group != "wheel"
                    && !groups.iter().any(|group| group == "wheel")
                {
                    return Err(invalid("cannot remove your own administrator membership"));
                }
            }
            run(
                "/usr/bin/usermod",
                &[
                    "--comment",
                    &name,
                    "--shell",
                    &shell,
                    "--gid",
                    &primary_group,
                    "--groups",
                    &groups.join(","),
                    "--",
                    &user,
                ],
            )?;
        }
        Request::Password {
            user,
            old,
            new,
            confirm,
        } => {
            let info = target(&user)?;
            if new != confirm {
                return Err(invalid("new password and confirmation do not match"));
            }
            password::validate_new_password(&new)?;
            if info.uid == actor_uid()? && !password::verify_own_password(&user, &old)? {
                return Err(Error::PermissionDenied(
                    "current password does not match".into(),
                ));
            }
            password::set_password_admin(&user, &new)?;
        }
        Request::Lock { user, locked } => {
            protect_account(&user)?;
            run(
                "/usr/bin/usermod",
                &[if locked { "--lock" } else { "--unlock" }, "--", &user],
            )?;
        }
        Request::ExpirePassword { user } => {
            target(&user)?;
            run("/usr/bin/chage", &["--lastday", "0", "--", &user])?;
        }
        Request::Delete { user } => {
            protect_account(&user)?;
            // Retain the home directory and never force deletion of logged-in users.
            run("/usr/bin/userdel", &["--", &user])?;
        }
        Request::CreateGroup { group } => {
            validation::validate_group_name(&group)?;
            run("/usr/bin/groupadd", &["--", &group])?;
        }
        Request::Avatar { user, path } => {
            let manager = AccountManager::new();
            if path.is_empty() {
                manager.remove_avatar(&user)?;
            } else {
                manager.set_avatar(&user, std::path::Path::new(&path))?;
            }
        }
    }
    Ok(json!({"ok": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_rejects_unknown_operations_and_fields() {
        assert!(serde_json::from_str::<Request>(r#"{"action":"shell","command":"id"}"#).is_err());
        assert!(
            serde_json::from_str::<Request>(r#"{"action":"delete","user":"root","force":true}"#)
                .is_err()
        );
    }
    #[test]
    fn protects_root_and_system_users() {
        assert!(protect_account("root").is_err());
        assert!(protect_account("../etc/passwd").is_err());
    }

    #[test]
    fn create_uses_shadow_argument_vector_and_leaves_password_locked() {
        let mut calls = Vec::new();
        let request = Request::Create {
            user: "argvus_test_created".into(),
            name: "Alice $(id)".into(),
            shell: shells().unwrap()[0].clone(),
            groups: Vec::new(),
        };
        apply_request(request, &mut |program, args| {
            calls.push((
                program.to_string(),
                args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>(),
            ));
            Ok(())
        })
        .unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "/usr/bin/useradd");
        assert!(calls[0].1.contains(&"--create-home".into()));
        assert!(calls[0].1.contains(&"Alice $(id)".into()));
        assert!(!calls[0].1.contains(&"--password".into()));
        assert_eq!(calls[0].1.last().unwrap(), "argvus_test_created");
    }

    #[test]
    fn invalid_creation_never_reaches_shadow_tools() {
        let request = Request::Create {
            user: "--root".into(),
            name: "Invalid".into(),
            shell: "/bin/sh".into(),
            groups: Vec::new(),
        };
        assert!(
            apply_request(request, &mut |_, _| panic!("must not run a system command")).is_err()
        );
    }

    #[test]
    fn cannot_delete_root_even_after_authorization() {
        assert!(
            apply_request(
                Request::Delete {
                    user: "root".into()
                },
                &mut |_, _| panic!("must not delete root")
            )
            .is_err()
        );
    }
}
