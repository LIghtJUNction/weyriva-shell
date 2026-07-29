#[path = "support/migration.rs"]
mod support;

use std::fs;

use support::{MigrationFixture, assert_absent};
use weyriva::legacy_migration::{MigrationOutcome, migrate};
use weyriva::tree::validate_and_hash;

#[test]
fn no_legacy_state_is_a_noop() {
    let fixture = MigrationFixture::empty();

    assert_eq!(
        migrate(&fixture.paths).expect("no-op migration should succeed"),
        MigrationOutcome::NotNeeded
    );
    assert_absent(&fixture.new);
    assert_absent(&fixture.staged);
    assert_absent(&fixture.recovery);
    assert!(!fixture.journal.exists());
}

#[test]
fn clean_migration_hydrates_both_lifecycle_states_and_preserves_modes() {
    let mut fixture = MigrationFixture::legacy();
    let enabled_digest = fixture.add_plugin("test/enabled", true, "Hydrated");
    fixture.add_plugin("test/disabled", false, "Disabled");
    let stale_record = fixture
        .state
        .plugins
        .get_mut("test/enabled")
        .expect("enabled record should exist");
    stale_record.provider.categories.clear();
    stale_record.provider.prefix = "stale".to_owned();
    stale_record.settings_defaults.clear();
    fixture.write_state();

    assert_eq!(
        migrate(&fixture.paths).expect("migration should succeed"),
        MigrationOutcome::Migrated
    );
    let state = fixture.migrated_state();
    let enabled = &state.plugins["test/enabled"];
    let disabled = &state.plugins["test/disabled"];
    assert!(enabled.enabled);
    assert!(!disabled.enabled);
    assert_eq!(enabled.provider.prefix, "enabled");
    assert_eq!(enabled.provider.categories[0].label, "Hydrated");
    assert_eq!(enabled.settings_defaults["uppercase"], true);
    assert_eq!(
        enabled.path,
        fixture.new[2]
            .join("installed/test/enabled")
            .join(&enabled_digest)
    );
    assert!(fixture.recovery.iter().all(|path| path.is_dir()));
    assert_absent(&fixture.old);
    assert_absent(&fixture.staged);
    assert!(!fixture.journal.exists());
    assert!(
        fixture
            .new
            .iter()
            .all(|path| MigrationFixture::mode(path) == 0o700)
    );
    assert_eq!(MigrationFixture::mode(&fixture.paths.state_file()), 0o600);
    assert_eq!(MigrationFixture::mode(&enabled.path), 0o555);
    assert_eq!(
        MigrationFixture::mode(&enabled.path.join("plugin.toml")),
        0o444
    );

    assert_eq!(
        migrate(&fixture.paths).expect("repeat migration should succeed"),
        MigrationOutcome::NotNeeded
    );
}

#[test]
fn independently_created_optional_roots_are_migrated_as_empty() {
    let fixture = MigrationFixture::empty();
    fs::create_dir_all(&fixture.old[0]).expect("legacy config should be created");
    fs::write(
        fixture.old[0].join("sources.json"),
        "{\"schema\":1,\"sources\":[]}\n",
    )
    .expect("sources should be written");

    assert_eq!(
        migrate(&fixture.paths).expect("partial migration should succeed"),
        MigrationOutcome::Migrated
    );
    assert!(fixture.new.iter().all(|path| path.is_dir()));
    assert!(fixture.recovery[0].is_dir());
    assert!(!fixture.recovery[1].exists());
    assert!(!fixture.recovery[2].exists());
}

#[test]
fn state_and_data_can_migrate_without_a_config_root() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", false, "General");
    fs::remove_file(fixture.old[0].join("sources.json")).expect("sources should be removed");
    fs::remove_dir(&fixture.old[0]).expect("config root should be removed");

    assert_eq!(
        migrate(&fixture.paths).expect("state/data migration should succeed"),
        MigrationOutcome::Migrated
    );
    assert!(fixture.new[0].is_dir());
    assert!(!fixture.recovery[0].exists());
    assert!(fixture.recovery[1].is_dir());
    assert!(fixture.recovery[2].is_dir());
}

#[test]
fn data_without_state_is_rejected_as_inconsistent() {
    let fixture = MigrationFixture::legacy();
    fs::create_dir_all(fixture.old[2].join("installed/orphan"))
        .expect("orphan data should be created");
    fs::remove_dir(&fixture.old[1]).expect("empty state root should be removed");

    let error = migrate(&fixture.paths).expect_err("orphan data should fail");

    assert_eq!(error.code(), "legacy_migration_inconsistent");
    assert!(fixture.old[2].is_dir());
    assert_absent(&fixture.staged);
}

#[test]
fn empty_data_root_without_records_is_valid() {
    let fixture = MigrationFixture::legacy();

    let outcome = migrate(&fixture.paths).expect("empty data root should migrate");

    assert_eq!(outcome, MigrationOutcome::Migrated);
    assert!(fixture.new[2].is_dir());
}

#[test]
fn malformed_state_is_non_mutating() {
    let fixture = MigrationFixture::legacy();
    fs::write(fixture.old[1].join("state.json"), b"{").expect("malformed state should be written");

    let error = migrate(&fixture.paths).expect_err("malformed state should fail");

    assert_eq!(error.code(), "invalid_state");
    assert!(fixture.old.iter().all(|path| path.is_dir()));
    assert_absent(&fixture.staged);
    assert_absent(&fixture.recovery);
}

#[test]
fn existing_unversioned_root_is_an_explicit_conflict() {
    let fixture = MigrationFixture::legacy();
    fs::create_dir_all(&fixture.new[0]).expect("conflict should be created");

    let error = migrate(&fixture.paths).expect_err("conflict should fail");

    assert_eq!(error.code(), "legacy_migration_conflict");
    assert!(fixture.old.iter().all(|path| path.is_dir()));
    assert_absent(&fixture.staged);
}

#[test]
fn state_requires_exact_id_path_and_existing_tree() {
    for defect in ["id", "path", "missing"] {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", false, "General");
        let record = fixture
            .state
            .plugins
            .get_mut("test/demo")
            .expect("record should exist");
        match defect {
            "id" => record.id = "test/other".to_owned(),
            "path" => record.path = fixture.old[2].join("installed/test/demo/not-the-digest"),
            "missing" => fs::remove_dir_all(&record.path).expect("tree should be removed"),
            _ => unreachable!(),
        }
        fixture.write_state();

        let error = migrate(&fixture.paths).expect_err("invalid record should fail");
        assert_eq!(error.code(), "legacy_migration_inconsistent", "{defect}");
        assert_absent(&fixture.staged);
    }
}

#[test]
fn actual_digest_and_manifest_id_must_match_state_key() {
    for defect in ["digest", "manifest"] {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", false, "General");
        let record = fixture
            .state
            .plugins
            .get_mut("test/demo")
            .expect("record should exist");
        if defect == "digest" {
            let wrong = "0".repeat(64);
            let wrong_slot = record
                .path
                .parent()
                .expect("slot should have parent")
                .join(&wrong);
            fs::rename(&record.path, &wrong_slot).expect("slot should be renamed");
            record.path = wrong_slot;
            record.digest = wrong;
        } else {
            let manifest = fs::read_to_string(record.path.join("plugin.toml"))
                .expect("manifest should be readable")
                .replace("id = \"test/demo\"", "id = \"other/demo\"");
            fs::write(record.path.join("plugin.toml"), manifest)
                .expect("manifest should be changed");
            let digest = validate_and_hash(&record.path).expect("changed tree should hash");
            let slot = record
                .path
                .parent()
                .expect("slot should have parent")
                .join(&digest);
            fs::rename(&record.path, &slot).expect("changed slot should be renamed");
            record.path = slot;
            record.digest = digest;
        }
        fixture.write_state();

        let error = migrate(&fixture.paths).expect_err("inconsistent tree should fail");
        assert_eq!(error.code(), "legacy_migration_inconsistent", "{defect}");
        assert_absent(&fixture.staged);
    }
}

#[test]
fn data_tree_is_closed_over_exact_state_references() {
    for defect in [
        "stray-slot",
        "stray-file",
        "top-level-file",
        "top-level-directory",
        "reference-gap",
        "duplicate-reference",
        "missing-lkg",
    ] {
        let mut fixture = MigrationFixture::legacy();
        fixture.add_plugin("test/demo", false, "General");
        match defect {
            "stray-slot" => {
                fs::create_dir_all(
                    fixture.old[2]
                        .join("installed/test/demo")
                        .join("0".repeat(64)),
                )
                .expect("stray slot should be created");
            }
            "stray-file" => {
                fs::write(fixture.old[2].join("installed/stray"), b"stray")
                    .expect("stray installed file should be written");
            }
            "top-level-file" => {
                fs::write(fixture.old[2].join("unreferenced"), b"stray")
                    .expect("stray top-level file should be written");
            }
            "top-level-directory" => {
                fs::create_dir(fixture.old[2].join("cache"))
                    .expect("stray top-level directory should be created");
            }
            "reference-gap" => {
                fixture.state.plugins.clear();
                fixture.write_state();
            }
            "duplicate-reference" => {
                let mut duplicate = fixture.state.plugins["test/demo"].clone();
                duplicate.id = "test/other".to_owned();
                fixture
                    .state
                    .plugins
                    .insert("test/other".to_owned(), duplicate);
                fixture.write_state();
            }
            "missing-lkg" => {
                fixture
                    .state
                    .plugins
                    .get_mut("test/demo")
                    .expect("record should exist")
                    .last_known_good = Some("0".repeat(64));
                fixture.write_state();
            }
            _ => unreachable!(),
        }

        let error = migrate(&fixture.paths).expect_err("open installed tree should fail");

        assert_eq!(error.code(), "legacy_migration_inconsistent", "{defect}");
        assert_absent(&fixture.staged);
        assert!(!fixture.journal.exists());
    }
}

#[test]
fn distinct_last_known_good_slot_is_a_valid_reference() {
    let mut fixture = MigrationFixture::legacy();
    fixture.add_plugin("test/demo", true, "General");
    let current = fixture.state.plugins["test/demo"].path.clone();
    let build = fixture.old[2].join(".lkg-build");
    fs::create_dir(&build).expect("LKG build directory should be created");
    let manifest = fs::read_to_string(current.join("plugin.toml"))
        .expect("current manifest should be readable")
        .replace("version = \"1.0.0\"", "version = \"0.9.0\"");
    fs::write(build.join("plugin.toml"), manifest).expect("LKG manifest should be written");
    fs::copy(current.join("main.luau"), build.join("main.luau"))
        .expect("LKG entry should be copied");
    let digest = validate_and_hash(&build).expect("LKG tree should hash");
    let slot = fixture.old[2].join("installed/test/demo").join(&digest);
    fs::rename(&build, &slot).expect("LKG tree should move into its immutable slot");
    fixture
        .state
        .plugins
        .get_mut("test/demo")
        .expect("record should exist")
        .last_known_good = Some(digest.clone());
    fixture.write_state();

    migrate(&fixture.paths).expect("referenced LKG slot should migrate");

    assert!(
        fixture.new[2]
            .join("installed/test/demo")
            .join(digest)
            .is_dir()
    );
}
