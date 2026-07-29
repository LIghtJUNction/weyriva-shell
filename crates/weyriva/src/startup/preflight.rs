use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::identity::UserAccount;
use crate::process::{CommandSpec, command_text, find_command, os};

use super::display_record;
use super::model::{
    DisplayManagerState, NIRI_WANTED_UNITS, REQUIRED_COMMANDS, StartupContext, StartupPlan,
    WEYRIVA_UNITS,
};
use super::safety::{validate_destination, validate_regular_file, validate_safe_chain};
use super::time::startup_timestamp;

const WALLPAPERS: &[&str] = &["light/weyriva-cactus.png", "dark/weyriva-cactus-dark.png"];
pub(super) const EXPECTED_DISPLAY_MANAGER: &str = "/usr/lib/systemd/system/greetd.service";
const LEGACY_MARKERS: &[(&str, &[&str])] = &[
    (
        "weyriva-waybar.service",
        &[
            "ExecStart=/usr/bin/waybar",
            "ExecStart=%h/.local/bin/weyriva component waybar",
        ],
    ),
    (
        "weyriva-mako.service",
        &[
            "ExecStart=/usr/bin/mako",
            "ExecStart=%h/.local/bin/weyriva component mako",
        ],
    ),
    (
        "weyriva-wallpaper.service",
        &["ExecStart=%h/.local/bin/weyriva wallpaper"],
    ),
    (
        "weyriva-idle.service",
        &["ExecStart=%h/.local/bin/weyriva idle"],
    ),
    (
        "weyriva-ipc.service",
        &[
            "ExecStart=/usr/bin/weyriva daemon",
            "ExecStart=%h/.local/bin/weyriva daemon",
        ],
    ),
    (
        "weyriva-shell.service",
        &[
            "ExecStart=/usr/bin/weyriva shell run",
            "ExecStart=%h/.local/bin/weyriva shell run",
        ],
    ),
    (
        "weyriva-session-failsafe.service",
        &["ExecStart=/usr/bin/niri msg action quit --skip-confirmation"],
    ),
];

/// Builds a complete, mutation-free startup reconciliation plan.
///
/// # Errors
///
/// Returns an error when identity, templates, paths, commands, or backups are unsafe.
pub fn preflight(context: &StartupContext, target_user: &str) -> Result<StartupPlan> {
    if context.identity.effective_uid() != 0 {
        return Err(Error::new(
            "root_required",
            "startup ensure requires root; run: sudo weyriva startup ensure",
        ));
    }
    if target_user.is_empty() || target_user == "root" {
        return Err(Error::new(
            "invalid_user",
            "cannot infer the desktop user; use --user USER",
        ));
    }
    let account = context
        .identity
        .user(target_user)?
        .ok_or_else(|| Error::new("invalid_user", format!("unknown user: {target_user}")))?;
    if account.uid == 0 {
        return Err(Error::new("invalid_user", "desktop user must be non-root"));
    }
    validate_required_files(context)?;
    validate_commands(context)?;
    validate_templates(context)?;
    validate_pam(&context.layout.greetd_pam)?;
    let greeter = context.identity.user("greeter")?.ok_or_else(|| {
        Error::new(
            "invalid_greeter",
            "the greeter account and group are required",
        )
    })?;
    let greeter_gid = context.identity.group_gid("greeter")?.ok_or_else(|| {
        Error::new(
            "invalid_greeter",
            "the greeter account and group are required",
        )
    })?;
    if greeter.gid != greeter_gid {
        return Err(Error::new(
            "invalid_greeter",
            "the greeter account must use the greeter group",
        ));
    }
    build_plan(context, target_user, account, (greeter.uid, greeter_gid))
}

fn build_plan(
    context: &StartupContext,
    target_user: &str,
    account: UserAccount,
    greeter_identity: (u32, u32),
) -> Result<StartupPlan> {
    validate_safe_chain(&account.home, true)?;
    validate_safe_chain(
        context
            .layout
            .greeter_state
            .parent()
            .unwrap_or(Path::new("/")),
        true,
    )?;
    validate_safe_chain(&context.layout.greeter_state, false)?;

    let packaged_niri = context.layout.packaged_config.join("niri/config.kdl");
    let niri_config = account.home.join(".config/niri/config.kdl");
    validate_destination(&niri_config)?;
    validate_niri(context, &packaged_niri)?;
    if niri_config.is_file() {
        validate_niri(context, &niri_config)?;
    }
    let niri_changed =
        !niri_config.is_file() || fs::read(&niri_config).ok() != fs::read(&packaged_niri).ok();

    validate_destination(&context.layout.greetd_config)?;
    let greetd_changed = !context.layout.greetd_config.is_file()
        || fs::read(&context.layout.greetd_config).ok()
            != fs::read(&context.layout.greetd_template).ok();
    let timestamp = startup_timestamp(
        &context.environment,
        context.clock.as_ref(),
        context.timezone.as_ref(),
    )?;
    let backup_root = account
        .home
        .join(".local/state/weyriva/startup-backups")
        .join(timestamp);
    validate_safe_chain(&backup_root, false)?;
    let niri_backup = backup_root.join("niri/config.kdl");
    let greetd_backup = backup_root.join("greetd/config.toml");
    let unit_root = account.home.join(".config/systemd/user");
    validate_safe_chain(&unit_root, false)?;
    let units_to_back_up = legacy_units(&unit_root);
    let display_manager_before = display_manager_state(context)?;
    let (display_manager_record, create_display_manager_record) = match &display_manager_before {
        DisplayManagerState::Absent => (None, false),
        DisplayManagerState::Link(target) => {
            let record = backup_root.join("systemd/display-manager.service");
            let existing = display_record::validate_existing(
                context.filesystem.as_ref(),
                &record,
                &context.layout.display_manager,
            )?;
            let create =
                !display_record::resolves_to_greetd(&context.layout.display_manager, target)?;
            ((existing || create).then_some(record), create)
        }
    };
    let mut backup_targets: Vec<PathBuf> = units_to_back_up
        .iter()
        .map(|name| backup_root.join("systemd/user").join(name))
        .collect();
    if niri_changed && niri_config.exists() {
        backup_targets.push(niri_backup.clone());
    }
    if greetd_changed && context.layout.greetd_config.exists() {
        backup_targets.push(greetd_backup.clone());
    }
    if let Some(conflict) = backup_targets
        .iter()
        .find(|path| fs::symlink_metadata(path).is_ok())
    {
        return Err(Error::new(
            "backup_collision",
            format!(
                "startup backup destination already exists: {}",
                conflict.display()
            ),
        ));
    }
    Ok(StartupPlan {
        target_user: target_user.to_owned(),
        user_home: account.home,
        user_uid: account.uid,
        user_gid: account.gid,
        greeter_uid: greeter_identity.0,
        greeter_gid: greeter_identity.1,
        niri_config,
        niri_backup,
        niri_changed,
        greetd_backup,
        greetd_changed,
        backup_root,
        unit_root,
        units_to_back_up,
        display_manager_before,
        display_manager_record,
        create_display_manager_record,
    })
}

fn validate_required_files(context: &StartupContext) -> Result<()> {
    let layout = &context.layout;
    for (path, description) in [
        (&layout.session_entry, "session entry"),
        (&layout.greetd_template, "greetd template"),
        (&layout.greeter_env, "greeter environment executable"),
        (&layout.greeter_session, "greeter session executable"),
        (&layout.packaged_shell.join("shell.qml"), "shell entry"),
        (&layout.packaged_greeter.join("shell.qml"), "greeter entry"),
        (
            &layout.packaged_config.join("niri/config.kdl"),
            "Niri template",
        ),
    ] {
        validate_regular_file(path, description)?;
    }
    for wallpaper in WALLPAPERS {
        validate_regular_file(
            &layout.packaged_data.join("wallpapers").join(wallpaper),
            "required wallpaper",
        )?;
    }
    for unit in WEYRIVA_UNITS {
        validate_regular_file(&layout.packaged_units.join(unit), "Weyriva user unit")?;
    }
    for unit in NIRI_WANTED_UNITS {
        let link = layout.packaged_units.join("niri.service.wants").join(unit);
        let target = fs::read_link(&link).map_err(|_| {
            Error::new(
                "startup_incomplete",
                format!("invalid niri service dependency: {}", link.display()),
            )
        })?;
        if target != Path::new(&format!("../{unit}")) {
            return Err(Error::new(
                "startup_incomplete",
                format!("invalid niri service dependency: {}", link.display()),
            ));
        }
    }
    Ok(())
}

fn validate_commands(context: &StartupContext) -> Result<()> {
    let path = context
        .environment
        .get(OsStr::new("PATH"))
        .map(OsString::as_os_str);
    let missing: Vec<&str> = REQUIRED_COMMANDS
        .iter()
        .copied()
        .filter(|command| find_command(command, path).is_none())
        .collect();
    if !missing.is_empty() {
        return Err(Error::new(
            "command_missing",
            format!("required commands are unavailable: {}", missing.join(", ")),
        ));
    }
    let output = context.process.run(&CommandSpec::new(
        "systemctl",
        [os("cat"), os("greetd.service")],
    ))?;
    if output.code != 0 {
        return Err(Error::new(
            "startup_incomplete",
            format!("greetd service is unavailable: {}", command_text(&output)),
        ));
    }
    Ok(())
}

fn validate_templates(context: &StartupContext) -> Result<()> {
    let session = fs::read_to_string(&context.layout.session_entry)
        .map_err(|error| Error::io("cannot read session entry", &error))?;
    if !session.contains("Name=Weyriva") || !session.contains("Exec=/usr/bin/weyriva session start")
    {
        return Err(Error::new(
            "startup_incomplete",
            "invalid Weyriva session entry",
        ));
    }
    let greetd = fs::read_to_string(&context.layout.greetd_template)
        .map_err(|error| Error::io("cannot read greetd template", &error))?;
    let markers = [
        "HOME=/var/lib/weyriva-greeter",
        "XDG_STATE_HOME=/var/lib/weyriva-greeter/state",
        "XDG_CACHE_HOME=/var/lib/weyriva-greeter/cache",
        "XDG_CONFIG_HOME=/var/lib/weyriva-greeter/config",
        "/usr/bin/cage -s -- /usr/bin/quickshell --path /usr/share/weyriva/greeter",
        "user = \"greeter\"",
    ];
    if greetd.contains("tuigreet") || !markers.iter().all(|marker| greetd.contains(marker)) {
        return Err(Error::new(
            "startup_incomplete",
            "invalid Weyriva greeter template",
        ));
    }
    Ok(())
}

fn validate_pam(path: &Path) -> Result<()> {
    validate_regular_file(path, "greetd PAM stack")?;
    let content = fs::read_to_string(path)
        .map_err(|error| Error::io("cannot read greetd PAM stack", &error))?;
    if content.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && trimmed
                .split_whitespace()
                .next()
                .is_some_and(|field| field.trim_start_matches('-') == "session")
            && trimmed.split_whitespace().count() >= 3
    }) {
        Ok(())
    } else {
        Err(Error::new(
            "startup_incomplete",
            "greetd PAM stack has no active session rule or include",
        ))
    }
}

fn validate_niri(context: &StartupContext, path: &Path) -> Result<()> {
    let output = context.process.run(&CommandSpec::new(
        "niri",
        [os("validate"), os("-c"), path.as_os_str().to_os_string()],
    ))?;
    if output.code == 0 {
        Ok(())
    } else {
        Err(Error::new(
            "invalid_niri_config",
            format!(
                "Niri config is invalid ({}): {}",
                path.display(),
                command_text(&output)
            ),
        ))
    }
}

fn legacy_units(root: &Path) -> Vec<String> {
    LEGACY_MARKERS
        .iter()
        .filter_map(|(name, markers)| {
            let path = root.join(name);
            if path.is_symlink() || !path.is_file() {
                return None;
            }
            fs::read_to_string(path)
                .ok()
                .filter(|content| markers.iter().any(|marker| content.contains(marker)))
                .map(|_| (*name).to_owned())
        })
        .collect()
}

fn display_manager_state(context: &StartupContext) -> Result<DisplayManagerState> {
    match fs::symlink_metadata(&context.layout.display_manager) {
        Ok(metadata) if !metadata.file_type().is_symlink() => Err(Error::new(
            "unsafe_startup_path",
            format!(
                "unsafe display-manager destination: {}",
                context.layout.display_manager.display()
            ),
        )),
        Ok(_) => {
            let target = fs::read_link(&context.layout.display_manager)
                .map_err(|error| Error::io("cannot read display-manager link", &error))?;
            Ok(DisplayManagerState::Link(target))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(DisplayManagerState::Absent)
        }
        Err(error) => Err(Error::io("cannot inspect display-manager link", &error)),
    }
}
