use std::path::PathBuf;

use tempfile::TempDir;
use weyriva::{Broker, Paths};

use crate::common;

pub struct InstalledFixture {
    pub paths: Paths,
    pub host: PathBuf,
}

pub fn installed_fixture(temporary: &TempDir, enabled: bool) -> InstalledFixture {
    let paths = common::paths(temporary);
    let source = temporary.path().join("source");
    common::write_plugin(&source, "1.0.0", "");
    let host = common::install_fake_host(temporary.path());
    let mut broker = Broker::with_host(paths.clone(), host.clone());
    broker
        .source_add("fixture", &source)
        .expect("fixture source should be added");
    broker
        .install("test/demo")
        .expect("fixture plugin should install");
    if enabled {
        broker
            .enable("test/demo")
            .expect("fixture plugin should enable");
    }
    InstalledFixture { paths, host }
}
