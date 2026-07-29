use std::fs;
use std::path::Path;

const LEGACY_UNIT_MARKERS: &[(&str, &[&str])] = &[
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

pub(super) fn legacy_overrides(user_units: &Path, packaged_units: &Path) -> Vec<&'static str> {
    LEGACY_UNIT_MARKERS
        .iter()
        .filter_map(|(name, markers)| {
            if !packaged_units.join(name).is_file() {
                return None;
            }
            let path = user_units.join(name);
            let metadata = fs::symlink_metadata(&path).ok()?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return None;
            }
            let content = fs::read_to_string(path).ok()?;
            markers
                .iter()
                .any(|marker| content.contains(marker))
                .then_some(*name)
        })
        .collect()
}
