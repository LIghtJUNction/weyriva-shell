#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir};
use weyriva::Paths;
use weyriva::manifest::parse_plugin;
use weyriva::model::{PluginRecord, Provenance, STATE_SCHEMA, StateDocument};
use weyriva::tree::validate_and_hash;

pub struct MigrationFixture {
    _temporary: TempDir,
    pub paths: Paths,
    pub old: [PathBuf; 3],
    pub new: [PathBuf; 3],
    pub staged: [PathBuf; 3],
    pub recovery: [PathBuf; 3],
    pub journal: PathBuf,
    pub state: StateDocument,
}

impl MigrationFixture {
    pub fn empty() -> Self {
        let temporary = tempdir().expect("temporary directory should be created");
        let root = temporary.path();
        let paths = Paths::new(
            &root.join("config"),
            &root.join("state"),
            &root.join("data"),
            &root.join("runtime"),
        );
        let old = siblings(&paths, "plugins-v5");
        let new = [
            paths.config_dir.clone(),
            paths.state_dir.clone(),
            paths.data_dir.clone(),
        ];
        let staged = siblings(&paths, ".plugins.migrating");
        let recovery = siblings(&paths, "plugins-v5.migrated");
        let journal = paths
            .state_dir
            .parent()
            .expect("state root should have a parent")
            .join(".plugins-migration.json");
        Self {
            _temporary: temporary,
            paths,
            old,
            new,
            staged,
            recovery,
            journal,
            state: StateDocument::default(),
        }
    }

    pub fn legacy() -> Self {
        let fixture = Self::empty();
        for root in &fixture.old {
            fs::create_dir_all(root).expect("legacy root should be created");
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .expect("legacy root should be private");
        }
        fs::write(
            fixture.old[0].join("sources.json"),
            "{\"schema\":1,\"sources\":[]}\n",
        )
        .expect("legacy sources should be written");
        fixture
    }

    pub fn add_plugin(&mut self, id: &str, enabled: bool, category: &str) -> String {
        let build = self.old[2].join(format!(".build-{}", self.state.plugins.len()));
        fs::create_dir(&build).expect("plugin build root should be created");
        let name = id.rsplit_once('/').map_or("Plugin", |(_, plugin)| plugin);
        let manifest = format!(
            r#"
id = "{id}"
name = "{name}"
version = "1.0.0"
plugin_api = 3
author = "Weyriva"

[[launcher_provider]]
id = "main"
entry = "main.luau"
prefix = "{name}"
glyph = "search"
include_in_global_search = true
debounce_ms = 80

[[launcher_provider.category]]
label = "{category}"
glyph = "folder"

[[setting]]
key = "uppercase"
type = "bool"
default = true
label_key = "Uppercase"
"#
        );
        fs::write(build.join("plugin.toml"), manifest).expect("plugin manifest should be written");
        fs::write(build.join("main.luau"), "return {}\n").expect("plugin entry should be written");
        let digest = validate_and_hash(&build).expect("fixture plugin should hash");
        let slot = self.old[2].join("installed").join(id).join(&digest);
        fs::create_dir_all(slot.parent().expect("slot parent should exist"))
            .expect("slot parent should be created");
        fs::rename(&build, &slot).expect("plugin should move into immutable slot");
        let candidate = parse_plugin(&slot).expect("fixture manifest should parse");
        let record = PluginRecord {
            id: id.to_owned(),
            installed: true,
            enabled,
            path: slot,
            digest: digest.clone(),
            version: candidate.provider.version.clone(),
            provider: candidate.provider,
            settings_defaults: candidate.settings_defaults,
            provenance: Provenance {
                source: "fixture".to_owned(),
                kind: "path".to_owned(),
                path: "/fixture".to_owned(),
                revision: None,
            },
            last_known_good: enabled.then(|| digest.clone()),
        };
        self.state.plugins.insert(id.to_owned(), record);
        self.write_state();
        digest
    }

    pub fn write_state(&self) {
        let mut state = self.state.clone();
        state.schema = STATE_SCHEMA;
        fs::write(
            self.old[1].join("state.json"),
            serde_json::to_vec(&state).expect("state should serialize"),
        )
        .expect("state should be written");
    }

    pub fn migrated_state(&self) -> StateDocument {
        serde_json::from_slice(
            &fs::read(self.paths.state_file()).expect("migrated state should be readable"),
        )
        .expect("migrated state should parse")
    }

    pub fn mode(path: &Path) -> u32 {
        fs::symlink_metadata(path)
            .expect("path should exist")
            .permissions()
            .mode()
            & 0o777
    }
}

fn siblings(paths: &Paths, name: &str) -> [PathBuf; 3] {
    [
        sibling(&paths.config_dir, name),
        sibling(&paths.state_dir, name),
        sibling(&paths.data_dir, name),
    ]
}

fn sibling(path: &Path, name: &str) -> PathBuf {
    path.parent()
        .expect("plugin root should have a parent")
        .join(name)
}

pub fn assert_absent(paths: &[PathBuf; 3]) {
    assert!(paths.iter().all(|path| !path.exists()), "{paths:?}");
}

pub fn blank_state() -> StateDocument {
    StateDocument {
        schema: STATE_SCHEMA,
        plugins: BTreeMap::new(),
    }
}
