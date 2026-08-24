//! Command dispatch and presentation logic.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use argvus_accounts_core::passwd;
use argvus_accounts_core::{AccountManager, Error, Result};

use crate::cli::{Cli, Command, SelfAction};

/// Executes the parsed command line.
pub fn execute(cli: &Cli) -> Result<()> {
    let manager = AccountManager::new();
    if cli.verbose {
        eprintln!(
            "argvus-accounts: authorization provider '{}', actor '{}'",
            manager.provider_name(),
            AccountManager::current_username().unwrap_or_else(|_| "?".to_string())
        );
    }

    match &cli.command {
        Command::List { all } => cmd_list(&manager, *all),
        Command::Show { user } => cmd_show(&manager, user),
        Command::Name { user, name } => {
            manager.set_display_name(user, name)?;
            println!("Display name updated successfully.");
            Ok(())
        }
        Command::Avatar {
            user,
            image,
            remove,
        } => cmd_avatar(&manager, user, image.as_deref(), *remove),
        Command::Groups { user, add, remove } => cmd_groups(&manager, user, add, remove),
        Command::SelfAccount { action } => cmd_self(&manager, action.as_ref(), cli.verbose),
    }
}

fn cmd_list(manager: &AccountManager, all: bool) -> Result<()> {
    let users = manager.list_users(all)?;
    println!("{:<17}NAME", "USER");
    for user in users {
        let name = user.display_name().unwrap_or("(none)");
        println!("{:<17}{}", user.username, name);
    }
    Ok(())
}

fn cmd_show(manager: &AccountManager, user: &str) -> Result<()> {
    let view = manager.get_user(user)?;
    let info = &view.info;
    println!("Username: {}", info.username);
    println!("Name: {}", view.display_name.as_deref().unwrap_or("(none)"));
    println!("UID: {}", info.uid);
    println!("GID: {}", info.gid);
    println!("Home: {}", info.home.display());
    println!("Shell: {}", info.shell);
    println!("Groups: {}", join_or_none(&view.groups));
    let avatar_line = view
        .avatar
        .as_deref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(none)".to_string());
    println!("Avatar: {avatar_line}");
    Ok(())
}

fn cmd_avatar(
    manager: &AccountManager,
    user: &str,
    image: Option<&Path>,
    remove: bool,
) -> Result<()> {
    if remove {
        manager.remove_avatar(user)?;
        println!("Avatar removed successfully.");
    } else {
        let image = image.expect("clap guarantees IMAGE when --remove is absent");
        let expanded = expand_tilde(image);
        manager.set_avatar(user, &expanded)?;
        println!("Avatar updated successfully.");
    }
    Ok(())
}

fn cmd_groups(
    manager: &AccountManager,
    user: &str,
    add: &[String],
    remove: &[String],
) -> Result<()> {
    if add.is_empty() && remove.is_empty() {
        let groups = manager.list_groups(user)?;
        println!("{}", join_or_none(&groups));
        return Ok(());
    }

    let remove_set: BTreeSet<&str> = remove.iter().map(String::as_str).collect();
    let overlap: Vec<&str> = add
        .iter()
        .map(String::as_str)
        .filter(|group| remove_set.contains(group))
        .collect();
    if !overlap.is_empty() {
        let names = overlap.join(", ");
        return Err(Error::InvalidOperation(format!(
            "cannot add and remove the same group(s) at once: {names}"
        )));
    }

    for group in add {
        manager.add_group(user, group)?;
        println!("Added '{user}' to group '{group}'.");
    }
    for group in remove {
        manager.remove_group(user, group)?;
        println!("Removed '{user}' from group '{group}'.");
    }
    Ok(())
}

fn cmd_self(manager: &AccountManager, action: Option<&SelfAction>, verbose: bool) -> Result<()> {
    let me = AccountManager::current_username()?;
    if verbose {
        eprintln!("argvus-accounts: self target '{me}'");
    }
    match action {
        None | Some(SelfAction::Show) => cmd_show(manager, &me),
        Some(SelfAction::Groups) => {
            let groups = manager.list_groups(&me)?;
            println!("{}", join_or_none(&groups));
            Ok(())
        }
        Some(SelfAction::Name { name }) => {
            manager.set_display_name(&me, name)?;
            println!("Display name updated successfully.");
            Ok(())
        }
        Some(SelfAction::Avatar { image, remove }) => {
            cmd_avatar(manager, &me, image.as_deref(), *remove)
        }
    }
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".to_string()
    } else {
        items.join(", ")
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if text == "~" {
        return home_dir().unwrap_or_else(|| path.to_path_buf());
    }
    if let Some(rest) = text.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }
    path.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| passwd::current_user().ok().map(|user| user.home))
}
