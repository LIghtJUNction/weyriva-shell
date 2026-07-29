use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};

use tempfile::tempdir;
use weyriva::startup::{Ownership, SystemOwnership};

#[test]
fn system_ownership_does_not_follow_a_dangling_symlink() {
    let temporary = tempdir().expect("temporary directory should be created");
    let link = temporary.path().join("dangling");
    symlink("missing-target", &link).expect("dangling link should be created");
    let metadata = fs::symlink_metadata(&link).expect("link metadata should be readable");

    SystemOwnership
        .set_owner(&link, metadata.uid(), metadata.gid())
        .expect("nofollow ownership should operate on the link itself");

    assert!(
        fs::symlink_metadata(&link)
            .expect("link should remain present")
            .file_type()
            .is_symlink()
    );
    assert!(!temporary.path().join("missing-target").exists());
}
