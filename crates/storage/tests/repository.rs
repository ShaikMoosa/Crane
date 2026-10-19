use crane_domain::{Store, SCHEMA_VERSION};
use crane_storage::{read_limited, write_atomic_bytes, write_atomic_bytes_with, JsonRepository};
use std::fs;
use std::io::{self, Read};
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
            .join("storage-tests")
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
fn empty_library_is_not_reseeded() {
    let directory = TestDirectory::new("empty");
    let path = directory.path().join("notes.json");
    let original = br#"{"schemaVersion":1,"notes":[]}"#;
    fs::write(&path, original).unwrap();

    let loaded = JsonRepository::new(path.clone()).load_or_seed(77).unwrap();

    assert_eq!(loaded.schema_version, SCHEMA_VERSION);
    assert!(loaded.notes.is_empty());
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn corruption_or_permission_error_never_seeds() {
    let directory = TestDirectory::new("load-errors");
    let corrupt = directory.path().join("corrupt.json");
    fs::write(&corrupt, b"not JSON").unwrap();
    let corrupt_before = fs::read(&corrupt).unwrap();

    assert!(JsonRepository::new(corrupt.clone())
        .load_or_seed(77)
        .is_err());
    assert_eq!(fs::read(corrupt).unwrap(), corrupt_before);

    let inaccessible = directory.path().join("directory-not-a-file.json");
    fs::create_dir(&inaccessible).unwrap();
    assert!(JsonRepository::new(inaccessible.clone())
        .load_or_seed(77)
        .is_err());
    assert!(inaccessible.is_dir());
}

#[test]
fn failed_replace_preserves_destination_and_owned_temp_cleanup() {
    let directory = TestDirectory::new("failed-replace");
    let path = directory.path().join("notes.json");
    let original = br#"{"schemaVersion":1,"notes":[]}"#;
    fs::write(&path, original).unwrap();

    let error = write_atomic_bytes_with(&path, b"replacement", |_, _| {
        Err(crane_platform::PlatformError::new(
            "injected replacement failure",
        ))
    })
    .expect_err("the injected replacement must fail");

    assert!(error.to_string().contains("injected replacement failure"));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(fs::read_dir(directory.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".crane-write-")
    }));
}

#[test]
fn unrelated_tmp_sibling_survives() {
    let directory = TestDirectory::new("unrelated-temp");
    let path = directory.path().join("notes.json");
    let unrelated = directory.path().join("notes.tmp");
    fs::write(&unrelated, b"keep me").unwrap();

    write_atomic_bytes(&path, br#"{"schemaVersion":1,"notes":[]}"#).unwrap();

    assert_eq!(
        fs::read(path).unwrap(),
        br#"{"schemaVersion":1,"notes":[]}"#
    );
    assert_eq!(fs::read(unrelated).unwrap(), b"keep me");
}

struct CountingReader {
    bytes: Vec<u8>,
    offset: usize,
    consumed: usize,
}

impl Read for CountingReader {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        let count = destination
            .len()
            .min(self.bytes.len().saturating_sub(self.offset));
        destination[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
        self.offset += count;
        self.consumed += count;
        Ok(count)
    }
}

#[test]
fn bounded_reader_stops_at_limit_plus_one() {
    let mut reader = CountingReader {
        bytes: vec![b'x'; 100],
        offset: 0,
        consumed: 0,
    };

    assert!(read_limited(&mut reader, 4).is_err());
    assert!(reader.consumed <= 5, "consumed {} bytes", reader.consumed);
}

#[test]
fn repository_round_trip_preserves_supported_extensions() {
    let directory = TestDirectory::new("round-trip");
    let path = directory.path().join("notes.json");
    let mut source: Store =
        serde_json::from_str(include_str!("../../../fixtures/compat/v1/extended.json")).unwrap();
    source.notes[0].title = "Changed".into();

    let mut repository = JsonRepository::new(path.clone());
    repository.commit(&source).unwrap();
    let loaded = repository.load_or_seed(88).unwrap();

    assert_eq!(
        serde_json::to_value(loaded).unwrap(),
        serde_json::to_value(source).unwrap()
    );
}
