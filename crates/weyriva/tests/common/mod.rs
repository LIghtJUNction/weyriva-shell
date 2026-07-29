#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use weyriva::Paths;

pub fn paths(temporary: &TempDir) -> Paths {
    let root = temporary.path();
    Paths::new(
        &root.join("config"),
        &root.join("state"),
        &root.join("data"),
        &root.join("runtime"),
    )
}

pub fn install_fake_host(root: &Path) -> PathBuf {
    let path = root.join("fake-host");
    fs::write(&path, include_bytes!("../fixtures/fake_host.py"))
        .expect("fake host fixture should be written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .expect("fake host fixture should be executable");
    path
}

pub fn write_plugin(source: &Path, version: &str, extra: &str) -> PathBuf {
    let plugin = source.join("demo-plugin");
    fs::create_dir_all(&plugin).expect("plugin fixture directory should be created");
    let manifest = format!(
        r#"
id = "test/demo"
name = "Demo"
version = "{version}"
plugin_api = 3
author = "Weyriva"
{extra}

[[launcher_provider]]
id = "main"
entry = "main.luau"
prefix = "demo"
glyph = "search"
include_in_global_search = true
debounce_ms = 80

[[launcher_provider.category]]
label = "General"
glyph = "folder"

[[setting]]
key = "uppercase"
type = "bool"
default = true
label_key = "Uppercase"
"#
    );
    fs::write(plugin.join("plugin.toml"), manifest)
        .expect("plugin manifest fixture should be written");
    fs::write(plugin.join("main.luau"), "return {}\n")
        .expect("plugin entry fixture should be written");
    plugin
}
