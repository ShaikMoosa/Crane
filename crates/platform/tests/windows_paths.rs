#![cfg(windows)]

use crane_platform::{replace_atomic, resolve_legacy_data_dir, LibraryGuard};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the test clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::current_dir()
            .expect("the test must have a working directory")
            .join("target")
            .join("platform-tests")
            .join(format!("{label}-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("the isolated test directory must be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn legacy_data_dir_uses_the_roaming_root_and_identifier() {
    let expected = dirs::data_dir().unwrap().join("app.crane.desktop");
    assert_eq!(resolve_legacy_data_dir().unwrap(), expected);
}

#[test]
fn atomic_replace_replaces_a_same_directory_destination() {
    let directory = TestDirectory::new("replace");
    let temporary = directory.path().join("temporary.tmp");
    let destination = directory.path().join("notes.json");
    fs::write(&temporary, b"new").unwrap();
    fs::write(&destination, b"old").unwrap();

    replace_atomic(&temporary, &destination).unwrap();

    assert_eq!(fs::read(destination).unwrap(), b"new");
    assert!(!temporary.exists());
}

#[test]
fn reparse_point_destination_is_refused() {
    let directory = TestDirectory::new("reparse");
    let target = directory.path().join("target-directory");
    let destination = directory.path().join("notes.json");
    let temporary = directory.path().join("temporary.tmp");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("old"), b"old").unwrap();
    fs::write(&temporary, b"new").unwrap();
    let junction = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&destination)
        .arg(&target)
        .output()
        .expect("the junction setup command must run");
    assert!(
        junction.status.success(),
        "junction setup failed: {}",
        String::from_utf8_lossy(&junction.stderr)
    );

    let error = replace_atomic(&temporary, &destination)
        .expect_err("a reparse-point destination must be refused");

    assert!(error.to_string().contains("reparse"));
    assert_eq!(fs::read(target.join("old")).unwrap(), b"old");
    assert_eq!(fs::read(temporary).unwrap(), b"new");
    fs::remove_dir(destination).unwrap();
}

#[test]
fn second_guard_is_rejected() {
    let directory = TestDirectory::new("guard");
    let first = LibraryGuard::acquire(directory.path()).unwrap();

    let error = LibraryGuard::acquire(directory.path())
        .expect_err("a second process-style handle must be rejected");

    assert!(error.to_string().contains("already in use"));
    drop(first);
    LibraryGuard::acquire(directory.path()).expect("dropping the guard must release the handle");
}
