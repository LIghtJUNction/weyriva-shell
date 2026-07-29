use std::collections::BTreeSet;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::StateOwnership;

#[derive(Debug, Eq, PartialEq)]
pub struct Snapshot {
    entries: BTreeSet<Entry>,
    enabled: bool,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Entry {
    path: PathBuf,
    kind: &'static str,
    mode: u32,
    owner: (u32, u32),
    content: Vec<u8>,
}

impl Snapshot {
    pub(super) fn capture(root: &Path, ownership: &StateOwnership, enabled: bool) -> Self {
        let mut entries = BTreeSet::new();
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            let entry = entry.expect("fixture tree should be readable");
            let path = entry.path();
            let metadata = fs::symlink_metadata(path).expect("metadata should be readable");
            let (kind, content) = if metadata.file_type().is_symlink() {
                (
                    "link",
                    fs::read_link(path)
                        .expect("link should be readable")
                        .as_os_str()
                        .as_bytes()
                        .to_vec(),
                )
            } else if metadata.is_file() {
                ("file", fs::read(path).expect("file should be readable"))
            } else {
                ("dir", Vec::new())
            };
            entries.insert(Entry {
                path: path
                    .strip_prefix(root)
                    .expect("entry should be beneath root")
                    .into(),
                kind,
                mode: metadata.permissions().mode() & 0o7777,
                owner: ownership.owner(path, &metadata),
                content,
            });
        }
        Self { entries, enabled }
    }
}
