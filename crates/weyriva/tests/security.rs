mod common;

use std::fs;
use std::io;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixListener;
use std::sync::atomic::AtomicBool;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::json;
use tar::{Builder, EntryType, Header};
use tempfile::tempdir;
use weyriva::archive;
use weyriva::host_session::HostSession;
use weyriva::manifest::parse_plugin;
use weyriva::model::Provider;
use weyriva::storage::atomic_json;
use weyriva::tree::validate_and_hash;
use weyriva::{Broker, ipc};

#[test]
fn manifest_exposes_categories_and_setting_defaults() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");

    let candidate = parse_plugin(&plugin).expect("valid API3 fixture should parse");

    assert_eq!(candidate.provider.categories[0].label, "General");
    assert_eq!(candidate.settings_defaults["uppercase"], json!(true));
}

#[test]
fn manifest_accepts_typed_api3_launcher_metadata() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = temporary.path().join("metadata-plugin");
    fs::create_dir(&plugin).expect("plugin fixture directory should be created");
    fs::write(
        plugin.join("plugin.toml"),
        r#"
id = "test/metadata"
name = "Metadata"
version = "1.0.0"
plugin_api = 3
author = "Weyriva"
deprecated = true

[[launcher_provider]]
id = "main"
entry = "main.luau"
prefix = "metadata"

[[setting]]
key = "document"
type = "file"
default = "/tmp/example.txt"
label_key = "Document"
advanced = true
visible_when = "enabled == true"
extensions = ["txt", "md"]
"#,
    )
    .expect("plugin manifest fixture should be written");
    fs::write(plugin.join("main.luau"), "return {}\n")
        .expect("plugin entry fixture should be written");

    let candidate = parse_plugin(&plugin).expect("typed API3 launcher metadata should parse");

    assert_eq!(
        candidate.settings_defaults["document"],
        json!("/tmp/example.txt")
    );
}

#[test]
fn manifest_rejects_mixed_plugin_surfaces() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = common::write_plugin(
        temporary.path(),
        "1.0.0",
        "\n[[widget]]\nid = \"mixed\"\nentry = \"widget.luau\"\n",
    );

    let error = parse_plugin(&plugin).expect_err("mixed surface should be rejected");

    assert_eq!(error.code(), "unsupported_plugin");
}

#[test]
fn manifest_rejects_symlinked_entry() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");
    fs::remove_file(plugin.join("main.luau")).expect("entry fixture should be removable");
    symlink("/dev/null", plugin.join("main.luau")).expect("entry symlink should be created");

    let error = parse_plugin(&plugin).expect_err("symlinked entry should be rejected");

    assert_eq!(error.code(), "invalid_manifest");
}

#[test]
fn archive_rejects_symlink_members() {
    let temporary = tempdir().expect("temporary directory should be created");
    let archive_path = temporary.path().join("source.tar.gz");
    let output = fs::File::create(&archive_path).expect("archive fixture should be created");
    let encoder = GzEncoder::new(output, Compression::default());
    let mut builder = Builder::new(encoder);
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Symlink);
    header.set_size(0);
    header
        .set_link_name("../escape")
        .expect("symlink target should be encoded");
    header.set_cksum();
    builder
        .append_data(&mut header, "root/link", io::empty())
        .expect("symlink member should be encoded");
    builder
        .into_inner()
        .expect("tar stream should finish")
        .finish()
        .expect("gzip stream should finish");

    let error = archive::extract(&archive_path, &temporary.path().join("out"))
        .expect_err("symlink member should be rejected");

    assert_eq!(error.code(), "unsafe_archive");
}

#[test]
fn archive_accepts_global_pax_header_metadata() {
    let temporary = tempdir().expect("temporary directory should be created");
    let archive_path = temporary.path().join("github-source.tar.gz");
    let output = fs::File::create(&archive_path).expect("archive fixture should be created");
    let encoder = GzEncoder::new(output, Compression::default());
    let mut builder = Builder::new(encoder);

    let pax_record = b"52 comment=0123456789012345678901234567890123456789\n";
    let mut pax_header = Header::new_gnu();
    pax_header.set_entry_type(EntryType::XGlobalHeader);
    pax_header.set_size(pax_record.len() as u64);
    pax_header.set_mode(0o644);
    pax_header.set_cksum();
    builder
        .append_data(&mut pax_header, "pax_global_header", pax_record.as_slice())
        .expect("global PAX header should be encoded");

    let mut directory_header = Header::new_gnu();
    directory_header.set_entry_type(EntryType::Directory);
    directory_header.set_size(0);
    directory_header.set_mode(0o755);
    directory_header.set_cksum();
    builder
        .append_data(&mut directory_header, "root/", io::empty())
        .expect("root directory should be encoded");

    let contents = b"plugin source\n";
    let mut file_header = Header::new_gnu();
    file_header.set_entry_type(EntryType::Regular);
    file_header.set_size(contents.len() as u64);
    file_header.set_mode(0o644);
    file_header.set_cksum();
    builder
        .append_data(&mut file_header, "root/main.luau", contents.as_slice())
        .expect("regular file should be encoded");
    builder
        .into_inner()
        .expect("tar stream should finish")
        .finish()
        .expect("gzip stream should finish");

    let extracted = archive::extract(&archive_path, &temporary.path().join("out"))
        .expect("global PAX metadata should be ignored");

    assert_eq!(
        fs::read(extracted.join("main.luau")).expect("extracted file should be readable"),
        contents
    );
}

#[test]
fn archive_rejects_excess_directory_members() {
    let temporary = tempdir().expect("temporary directory should be created");
    let archive_path = temporary.path().join("directory-bomb.tar.gz");
    let output = fs::File::create(&archive_path).expect("archive fixture should be created");
    let encoder = GzEncoder::new(output, Compression::default());
    let mut builder = Builder::new(encoder);
    for index in 0..2_049 {
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Directory);
        header.set_size(0);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("root/d{index}/"), io::empty())
            .expect("directory member should be encoded");
    }
    builder
        .into_inner()
        .expect("tar stream should finish")
        .finish()
        .expect("gzip stream should finish");

    let error = archive::extract(&archive_path, &temporary.path().join("out"))
        .expect_err("excess directory members should be rejected");

    assert_eq!(error.code(), "archive_too_large");
}

#[test]
fn archive_rejects_hardlink_members() {
    let temporary = tempdir().expect("temporary directory should be created");
    let archive_path = temporary.path().join("hardlink.tar.gz");
    let output = fs::File::create(&archive_path).expect("archive fixture should be created");
    let encoder = GzEncoder::new(output, Compression::default());
    let mut builder = Builder::new(encoder);
    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Link);
    header.set_size(0);
    header
        .set_link_name("root/target")
        .expect("hardlink target should be encoded");
    header.set_cksum();
    builder
        .append_data(&mut header, "root/link", io::empty())
        .expect("hardlink member should be encoded");
    builder
        .into_inner()
        .expect("tar stream should finish")
        .finish()
        .expect("gzip stream should finish");

    let error = archive::extract(&archive_path, &temporary.path().join("out"))
        .expect_err("archive hardlink should be rejected");

    assert_eq!(error.code(), "unsafe_archive");
}

#[test]
fn archive_rejects_multiple_roots() {
    let temporary = tempdir().expect("temporary directory should be created");
    let archive_path = temporary.path().join("multiple-roots.tar.gz");
    let output = fs::File::create(&archive_path).expect("archive fixture should be created");
    let encoder = GzEncoder::new(output, Compression::default());
    let mut builder = Builder::new(encoder);
    for path in ["root-a/", "root-b/"] {
        let mut header = Header::new_gnu();
        header.set_entry_type(EntryType::Directory);
        header.set_size(0);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, path, io::empty())
            .expect("root directory should be encoded");
    }
    builder
        .into_inner()
        .expect("tar stream should finish")
        .finish()
        .expect("gzip stream should finish");

    let error = archive::extract(&archive_path, &temporary.path().join("out"))
        .expect_err("multiple archive roots should be rejected");

    assert_eq!(error.code(), "unsafe_archive");
}

#[test]
fn plugin_tree_rejects_hardlinked_files() {
    let temporary = tempdir().expect("temporary directory should be created");
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");
    fs::hard_link(plugin.join("main.luau"), plugin.join("alias.luau"))
        .expect("hardlink fixture should be created");

    let error = validate_and_hash(&plugin).expect_err("hardlinked source should be rejected");

    assert_eq!(error.code(), "unsafe_plugin");
}

#[test]
fn paths_use_unversioned_product_namespace() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);

    assert!(paths.config_dir.ends_with("weyriva/plugins"));
    assert!(paths.state_dir.ends_with("weyriva/plugins"));
    assert!(paths.data_dir.ends_with("weyriva/plugins"));
}

#[test]
fn atomic_json_writes_private_state() {
    let temporary = tempdir().expect("temporary directory should be created");
    let path = temporary.path().join("state/state.json");

    atomic_json(&path, &json!({"schema": 1})).expect("state should be written");

    let mode = fs::metadata(path)
        .expect("state metadata should be readable")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn host_accepts_response_larger_than_request_limit() {
    let temporary = tempdir().expect("temporary directory should be created");
    let host = common::install_fake_host(temporary.path());
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");
    let provider = provider();
    let mut session =
        HostSession::start(&host, &plugin, &provider, &json!({})).expect("host should start");

    let response = session
        .request("query", json!({"query": "large"}))
        .expect("70 KiB response should be accepted");

    assert_eq!(
        response["padding"]
            .as_str()
            .expect("padding should be a string")
            .len(),
        70 * 1024
    );
}

#[test]
fn host_rejects_response_larger_than_one_mebibyte() {
    let temporary = tempdir().expect("temporary directory should be created");
    let host = common::install_fake_host(temporary.path());
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");
    let provider = provider();
    let mut session =
        HostSession::start(&host, &plugin, &provider, &json!({})).expect("host should start");

    let error = session
        .request("query", json!({"query": "oversize"}))
        .expect_err("response above one MiB should be rejected");

    assert_eq!(error.code(), "host_protocol");
}

#[test]
fn host_rejects_request_larger_than_64_kibibytes() {
    let temporary = tempdir().expect("temporary directory should be created");
    let host = common::install_fake_host(temporary.path());
    let plugin = common::write_plugin(temporary.path(), "1.0.0", "");
    let provider = provider();
    let mut session =
        HostSession::start(&host, &plugin, &provider, &json!({})).expect("host should start");

    let error = session
        .request("query", json!({"query": "x".repeat(70 * 1024)}))
        .expect_err("request above 64 KiB should be rejected");

    assert_eq!(error.code(), "host_protocol");
}

#[test]
fn control_client_rejects_request_larger_than_64_kibibytes() {
    let temporary = tempdir().expect("temporary directory should be created");
    let socket = temporary.path().join("control.sock");
    let _listener = UnixListener::bind(&socket).expect("control socket should bind");

    let error = ipc::call(
        &socket,
        "weyriva.plugin.v1.query",
        &json!({"provider": "test/demo:main", "query": "x".repeat(70 * 1024)}),
    )
    .expect_err("control request above 64 KiB should be rejected");

    assert_eq!(error.code(), "request_too_large");
}

#[test]
fn daemon_refuses_non_socket_endpoint() {
    let temporary = tempdir().expect("temporary directory should be created");
    let paths = common::paths(&temporary);
    fs::create_dir_all(&paths.runtime_dir).expect("runtime directory should be created");
    fs::write(paths.socket_file(), b"occupied").expect("unsafe endpoint should be created");
    let broker = Broker::with_host(paths.clone(), temporary.path().join("unused-host"));

    let error = ipc::serve_until(&paths, broker, &AtomicBool::new(true))
        .expect_err("daemon should refuse a non-socket endpoint");

    assert_eq!(error.code(), "unsafe_socket");
}

fn provider() -> Provider {
    Provider {
        plugin_id: "test/demo".to_owned(),
        entry_id: "main".to_owned(),
        entry: "main.luau".to_owned(),
        prefix: "demo".to_owned(),
        name: "Demo".to_owned(),
        version: "1.0.0".to_owned(),
        glyph: "search".to_owned(),
        include_in_global_search: true,
        debounce_ms: 80,
        categories: Vec::new(),
    }
}
