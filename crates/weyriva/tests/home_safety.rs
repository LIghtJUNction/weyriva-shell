use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use tempfile::tempdir;
use weyriva::session::niri_config_path;
use weyriva::shell::shell_root;

#[test]
fn missing_home_never_selects_cwd_relative_desktop_content() {
    let environment = BTreeMap::new();

    assert_eq!(
        shell_root(&environment),
        Path::new("/usr/share/weyriva/shell")
    );
    assert_eq!(
        niri_config_path(&environment),
        Path::new("/usr/share/weyriva/config/niri/config.kdl")
    );
}

#[test]
fn absolute_xdg_roots_work_without_home() {
    let temporary = tempdir().expect("temporary directory should be created");
    let shell = temporary.path().join("data/weyriva/shell");
    let niri = temporary.path().join("config/niri");
    std::fs::create_dir_all(&shell).expect("shell directory should be created");
    std::fs::create_dir_all(&niri).expect("Niri directory should be created");
    std::fs::write(shell.join("shell.qml"), "fixture\n").expect("shell fixture should be written");
    std::fs::write(niri.join("config.kdl"), "fixture\n").expect("Niri fixture should be written");
    let environment = BTreeMap::from([
        (OsString::from("HOME"), OsString::new()),
        (
            OsString::from("XDG_DATA_HOME"),
            temporary.path().join("data").into_os_string(),
        ),
        (
            OsString::from("XDG_CONFIG_HOME"),
            temporary.path().join("config").into_os_string(),
        ),
    ]);

    assert_eq!(shell_root(&environment), shell);
    assert_eq!(niri_config_path(&environment), niri.join("config.kdl"));
}

#[test]
fn relative_home_and_xdg_roots_never_select_cwd_content() {
    let environment = BTreeMap::from([
        (OsString::from("HOME"), OsString::from(".")),
        (OsString::from("XDG_DATA_HOME"), OsString::from("data")),
        (OsString::from("XDG_CONFIG_HOME"), OsString::from("config")),
    ]);

    assert_eq!(
        shell_root(&environment),
        Path::new("/usr/share/weyriva/shell")
    );
    assert_eq!(
        niri_config_path(&environment),
        Path::new("/usr/share/weyriva/config/niri/config.kdl")
    );
}
