use std::fmt::Write as _;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::process::{CommandSpec, command_text, os};

use super::display_record;
use super::model::{DisplayManagerState, StartupContext, StartupPlan};
use super::preflight::EXPECTED_DISPLAY_MANAGER;
use super::transaction::{EnableState, Transaction};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyResult {
    pub output: String,
    pub niri_changed: bool,
    pub greetd_changed: bool,
    pub moved_units: Vec<String>,
}

/// Applies a fully validated startup plan without restarting greetd.
///
/// # Errors
///
/// Returns an error when a planned filesystem, ownership, or enable operation fails.
pub fn apply(context: &StartupContext, plan: &StartupPlan) -> Result<ApplyResult> {
    let display_manager = display_manager_state(context)?;
    if display_manager != plan.display_manager_before {
        return Err(Error::new(
            "startup_plan_stale",
            "display-manager link changed after startup preflight",
        ));
    }
    let enable_state = greetd_enable_state(context)?;
    let target_owner = display_target_owner(context, &display_manager)?;
    let mut transaction = Transaction::new(context);
    match apply_transaction(
        &mut transaction,
        context,
        plan,
        display_manager,
        enable_state,
        target_owner.as_ref(),
    ) {
        Ok(result) => Ok(result),
        Err(error) => Err(transaction.rollback(error)),
    }
}

fn apply_transaction(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    plan: &StartupPlan,
    display_manager: DisplayManagerState,
    enable_state: EnableState,
    target_owner: Option<&TargetOwner>,
) -> Result<ApplyResult> {
    ensure_greeter_directories(transaction, context, plan)?;
    let packaged_niri = context.layout.packaged_config.join("niri/config.kdl");
    let user = (plan.user_uid, plan.user_gid);
    let niri_changed = transaction.replace_file(
        &packaged_niri,
        &plan.niri_config,
        &plan.niri_backup,
        user,
        user,
    )?;
    own_user_niri(transaction, context, plan)?;
    let greetd_changed = transaction.replace_file(
        &context.layout.greetd_template,
        &context.layout.greetd_config,
        &plan.greetd_backup,
        (0, 0),
        user,
    )?;
    let moved_units = back_up_units(transaction, plan)?;
    persist_display_manager(transaction, context, plan, &display_manager)?;
    own_backup_tree(transaction, context, plan)?;
    verify_target_owner(context, target_owner)?;
    enable_greetd(transaction, context, display_manager, enable_state)?;
    Ok(ApplyResult {
        output: render(plan, niri_changed, greetd_changed, &moved_units),
        niri_changed,
        greetd_changed,
        moved_units,
    })
}

fn persist_display_manager(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    plan: &StartupPlan,
    state: &DisplayManagerState,
) -> Result<()> {
    let (Some(path), DisplayManagerState::Link(target)) = (&plan.display_manager_record, state)
    else {
        return Ok(());
    };
    if display_record::validate_existing(
        context.filesystem.as_ref(),
        path,
        &context.layout.display_manager,
    )? {
        transaction.set_mode(path, 0o600)?;
        transaction.set_owner(path, (plan.user_uid, plan.user_gid))?;
        return Ok(());
    }
    if !plan.create_display_manager_record {
        return Ok(());
    }
    let data = display_record::encode(&context.layout.display_manager, target)?;
    transaction.write_new_file(path, &data, 0o600, (plan.user_uid, plan.user_gid))?;
    Ok(())
}

fn ensure_greeter_directories(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    plan: &StartupPlan,
) -> Result<()> {
    let greeter = (plan.greeter_uid, plan.greeter_gid);
    transaction.ensure_directory(&context.layout.greeter_state, 0o750, greeter)?;
    for name in ["state", "cache", "config"] {
        transaction.ensure_directory(&context.layout.greeter_state.join(name), 0o750, greeter)?;
    }
    Ok(())
}

fn own_user_niri(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    plan: &StartupPlan,
) -> Result<()> {
    for path in [
        plan.user_home.join(".config"),
        plan.niri_config
            .parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "Niri config has no parent"))?
            .to_path_buf(),
        plan.niri_config.clone(),
    ] {
        match context.filesystem.symlink_metadata(&path) {
            Ok(_) => transaction.set_owner(&path, (plan.user_uid, plan.user_gid))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("cannot inspect Niri ownership", &error)),
        }
    }
    Ok(())
}

fn back_up_units(transaction: &mut Transaction<'_>, plan: &StartupPlan) -> Result<Vec<String>> {
    let mut moved = Vec::new();
    for name in &plan.units_to_back_up {
        let source = plan.unit_root.join(name);
        let destination = plan.backup_root.join("systemd/user").join(name);
        let parent = destination
            .parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "unit backup has no parent"))?;
        transaction.create_tree(parent, 0o700, (plan.user_uid, plan.user_gid))?;
        transaction.rename(&source, &destination)?;
        moved.push(name.clone());
    }
    Ok(moved)
}

fn own_backup_tree(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    plan: &StartupPlan,
) -> Result<()> {
    match context.filesystem.symlink_metadata(&plan.backup_root) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Error::io("cannot inspect startup backup", &error)),
        Ok(_) => {}
    }
    for entry in walkdir::WalkDir::new(&plan.backup_root).follow_links(false) {
        let entry = entry.map_err(|error| Error::new("ownership_failed", error.to_string()))?;
        transaction.set_owner(entry.path(), (plan.user_uid, plan.user_gid))?;
    }
    Ok(())
}

fn enable_greetd(
    transaction: &mut Transaction<'_>,
    context: &StartupContext,
    display_manager: DisplayManagerState,
    enable_state: EnableState,
) -> Result<()> {
    transaction.journal_enable(
        enable_state,
        &context.layout.display_manager,
        display_manager,
    );
    let output = context.process.run(&CommandSpec::new(
        "systemctl",
        [os("enable"), os("--force"), os("greetd.service")],
    ))?;
    if output.code != 0 {
        return Err(Error::new(
            "startup_apply_failed",
            format!("enabling greetd failed: {}", command_text(&output)),
        ));
    }
    transaction.applied("systemctl_enable:greetd.service")?;
    if display_manager_state(context)?
        != DisplayManagerState::Link(PathBuf::from(EXPECTED_DISPLAY_MANAGER))
    {
        return Err(Error::new(
            "startup_apply_failed",
            "enabling greetd did not select the expected display manager",
        ));
    }
    if !matches!(greetd_enable_state(context)?, EnableState::Enabled) {
        return Err(Error::new(
            "startup_apply_failed",
            "greetd did not become effectively enabled",
        ));
    }
    Ok(())
}

fn greetd_enable_state(context: &StartupContext) -> Result<EnableState> {
    let output = context.process.run(&CommandSpec::new(
        "systemctl",
        [os("is-enabled"), os("greetd.service")],
    ))?;
    match command_text(&output).as_str() {
        "enabled" if output.code == 0 => Ok(EnableState::Enabled),
        "disabled" => Ok(EnableState::Disabled),
        state => Err(Error::new(
            "startup_apply_failed",
            format!("unsupported greetd enable state: {state}"),
        )),
    }
}

fn display_manager_state(context: &StartupContext) -> Result<DisplayManagerState> {
    match context
        .filesystem
        .symlink_metadata(&context.layout.display_manager)
    {
        Ok(metadata) if metadata.file_type().is_symlink() => context
            .filesystem
            .read_link(&context.layout.display_manager)
            .map(DisplayManagerState::Link)
            .map_err(|error| Error::io("cannot read display-manager link", &error)),
        Ok(_) => Err(Error::new(
            "unsafe_startup_path",
            "display-manager destination is not a symlink",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(DisplayManagerState::Absent),
        Err(error) => Err(Error::io("cannot inspect display-manager link", &error)),
    }
}

struct TargetOwner {
    path: PathBuf,
    uid: u32,
    gid: u32,
}

fn display_target_owner(
    context: &StartupContext,
    state: &DisplayManagerState,
) -> Result<Option<TargetOwner>> {
    let DisplayManagerState::Link(target) = state else {
        return Ok(None);
    };
    let path = if target.is_absolute() {
        target.clone()
    } else {
        context
            .layout
            .display_manager
            .parent()
            .ok_or_else(|| Error::new("unsafe_startup_path", "display-manager has no parent"))?
            .join(target)
    };
    let metadata = match context.filesystem.metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::io("cannot inspect display-manager target", &error)),
    };
    Ok(Some(TargetOwner {
        path,
        uid: metadata.uid(),
        gid: metadata.gid(),
    }))
}

fn verify_target_owner(context: &StartupContext, expected: Option<&TargetOwner>) -> Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let metadata = context
        .filesystem
        .metadata(&expected.path)
        .map_err(|error| Error::io("cannot verify display-manager target", &error))?;
    if (metadata.uid(), metadata.gid()) == (expected.uid, expected.gid) {
        Ok(())
    } else {
        Err(Error::new(
            "ownership_failed",
            "display-manager target ownership changed during startup apply",
        ))
    }
}

fn render(
    plan: &StartupPlan,
    niri_changed: bool,
    greetd_changed: bool,
    moved: &[String],
) -> String {
    let mut output = format!(
        "Weyriva Niri startup chain ensured for {}.\n\
         Niri config: {} ({})\n\
         greetd config: {}\n",
        plan.target_user,
        if niri_changed {
            "updated"
        } else {
            "already current"
        },
        plan.niri_config.display(),
        if greetd_changed {
            "updated"
        } else {
            "already current"
        },
    );
    if moved.is_empty() {
        output.push_str("legacy user units: none\n");
    } else {
        let _ = writeln!(
            output,
            "legacy user units backed up: {}\nbackup directory: {}",
            moved.join(", "),
            plan.backup_root.display()
        );
    }
    output.push_str("greetd was not restarted; log out or reboot when ready.\n");
    output
}
