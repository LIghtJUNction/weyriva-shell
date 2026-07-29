use std::collections::BTreeMap;
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use weyriva::startup::{FileSystem, SystemFileSystem};

pub struct TestFileSystem {
    inner: SystemFileSystem,
    fail_sync: Mutex<Option<PathBuf>>,
    reads: BTreeMap<PathBuf, Vec<u8>>,
}

impl TestFileSystem {
    pub fn failing_sync(path: PathBuf) -> Self {
        Self {
            inner: SystemFileSystem,
            fail_sync: Mutex::new(Some(path)),
            reads: BTreeMap::new(),
        }
    }

    pub fn readable(path: PathBuf, data: Vec<u8>) -> Self {
        Self {
            inner: SystemFileSystem,
            fail_sync: Mutex::new(None),
            reads: BTreeMap::from([(path, data)]),
        }
    }
}

impl FileSystem for TestFileSystem {
    fn symlink_metadata(&self, path: &Path) -> io::Result<Metadata> {
        self.inner.symlink_metadata(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<Metadata> {
        self.inner.metadata(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.reads
            .get(path)
            .cloned()
            .map_or_else(|| self.inner.read(path), Ok)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.read_link(path)
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_file(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.inner.rename(from, to)
    }

    fn set_mode(&self, path: &Path, mode: u32) -> io::Result<()> {
        self.inner.set_mode(path, mode)
    }

    fn write_atomic(&self, path: &Path, data: &[u8], mode: u32) -> io::Result<()> {
        self.inner.write_atomic(path, data, mode)
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        self.inner.symlink(target, link)
    }

    fn sync_directory(&self, path: &Path) -> io::Result<()> {
        let mut fail_sync = self
            .fail_sync
            .lock()
            .expect("sync failure lock should remain available");
        if fail_sync.as_deref() == Some(path) {
            fail_sync.take();
            return Err(io::Error::other("injected directory sync failure"));
        }
        self.inner.sync_directory(path)
    }
}
