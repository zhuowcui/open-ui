//! A polling file watcher for `openui dev`. An app's sources are small enough
//! that comparing modification times a few times a second costs little, and
//! it needs no platform-specific dependency.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

const POLL: Duration = Duration::from_millis(250);
/// Wait this long after the last change so a save of several files, or an
/// editor's write-then-rename, triggers one rebuild.
const SETTLE: Duration = Duration::from_millis(150);

/// File path to (modification time, length) for every watched file.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Snapshot(BTreeMap<PathBuf, (Option<SystemTime>, u64)>);

pub struct Watcher {
    roots: Vec<PathBuf>,
}

impl Watcher {
    /// Watch these files and directories; missing ones are watched for
    /// creation.
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut files = BTreeMap::new();
        for root in &self.roots {
            collect(root, &mut files);
        }
        Snapshot(files)
    }

    /// Block until the watched files differ from `since` and then stay
    /// unchanged for a moment. Returns at once if they already differ, so a
    /// change made during a build is not lost.
    pub fn wait_for_change(&self, since: &Snapshot) -> Snapshot {
        let mut current = self.snapshot();
        while current == *since {
            thread::sleep(POLL);
            current = self.snapshot();
        }
        loop {
            thread::sleep(SETTLE);
            let next = self.snapshot();
            if next == current {
                return next;
            }
            current = next;
        }
    }
}

fn collect(path: &Path, files: &mut BTreeMap<PathBuf, (Option<SystemTime>, u64)>) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        files.insert(
            path.to_path_buf(),
            (metadata.modified().ok(), metadata.len()),
        );
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !ignored(&name.to_string_lossy()) {
            collect(&entry.path(), files);
        }
    }
}

/// Build output, version control and editor scratch files.
fn ignored(name: &str) -> bool {
    name == "target"
        || name.starts_with('.')
        || name.ends_with('~')
        || name.ends_with(".swp")
        || name.ends_with(".tmp")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("openui-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("src")).unwrap();
        directory
    }

    #[test]
    fn snapshots_see_edits_new_files_and_deletions_but_not_ignored_files() {
        let directory = scratch("snapshot");
        let main = directory.join("src/main.rs");
        fs::write(&main, "fn main() {}").unwrap();
        let watcher = Watcher::new(vec![directory.join("src"), directory.join("missing")]);
        let first = watcher.snapshot();
        assert_eq!(first.0.len(), 1);

        fs::write(directory.join("src/.main.rs.swp"), "x").unwrap();
        fs::write(directory.join("src/main.rs~"), "x").unwrap();
        fs::create_dir_all(directory.join("src/target")).unwrap();
        fs::write(directory.join("src/target/out"), "x").unwrap();
        assert_eq!(watcher.snapshot(), first);

        fs::write(&main, "fn main() { println!(); }").unwrap();
        let edited = watcher.snapshot();
        assert_ne!(edited, first);

        fs::write(directory.join("src/lib.rs"), "").unwrap();
        let added = watcher.snapshot();
        assert_eq!(added.0.len(), 2);

        fs::remove_file(directory.join("src/lib.rs")).unwrap();
        assert_eq!(watcher.snapshot(), edited);

        fs::create_dir_all(directory.join("missing")).unwrap();
        fs::write(directory.join("missing/asset.png"), "x").unwrap();
        assert_eq!(watcher.snapshot().0.len(), 2);
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn wait_returns_at_once_for_a_change_made_before_waiting() {
        let directory = scratch("wait");
        let file = directory.join("src/main.rs");
        fs::write(&file, "a").unwrap();
        let watcher = Watcher::new(vec![directory.join("src")]);
        let before = watcher.snapshot();
        fs::write(&file, "ab").unwrap();
        let after = watcher.wait_for_change(&before);
        assert_ne!(after, before);
        assert_eq!(after, watcher.snapshot());
        fs::remove_dir_all(&directory).unwrap();
    }
}
