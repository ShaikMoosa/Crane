use crate::model::{Store, SCHEMA_VERSION};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn load_or_seed(path: &Path, timestamp: u64) -> Result<Store, String> {
    if !path.exists() {
        let store = Store::seed(timestamp);
        save(path, &store)?;
        return Ok(store);
    }
    let bytes =
        fs::read(path).map_err(|error| format!("Could not read the local store: {error}"))?;
    let store: Store = serde_json::from_slice(&bytes)
        .map_err(|error| format!("The local store is not valid JSON: {error}"))?;
    if store.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "Unsupported store version {}.",
            store.schema_version
        ));
    }
    Ok(store)
}

pub fn save(path: &Path, store: &Store) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|error| format!("Could not serialize notes: {error}"))?;
    write_atomic_bytes(path, &bytes)
}

pub(crate) fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "The data path has no parent directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create the data directory: {error}"))?;
    let (temporary, mut file) = create_temporary_file(parent)?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not write notes: {error}"));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("Could not flush notes to disk: {error}"));
    }
    drop(file);
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

fn create_temporary_file(parent: &Path) -> Result<(PathBuf, File), String> {
    for _ in 0..1_000 {
        let sequence = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".crane-write-{}-{sequence}.tmp",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("Could not create a temporary store: {error}"));
            }
        }
    }
    Err("Could not reserve a unique temporary store file.".into())
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(temporary, destination)
        .map_err(|error| format!("Could not replace the local store: {error}"))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        return Err(format!(
            "Could not replace the local store: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_store() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let folder = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!("crane-test-{}-{nonce}", std::process::id()));
        let path = folder.join("notes.json");
        let source = Store::seed(42);
        save(&path, &source).expect("save should succeed");
        let mut replacement = source.clone();
        replacement.notes.clear();
        save(&path, &replacement).expect("replacing an existing store should succeed");
        let loaded = load_or_seed(&path, 99).expect("load should succeed");
        assert!(loaded.notes.is_empty());
        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn atomic_save_never_reuses_or_removes_an_unrelated_tmp_sibling() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let folder = std::env::current_dir()
            .expect("the test should have a working directory")
            .join("target")
            .join(format!("crane-atomic-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&folder).expect("test folder should exist");
        let path = folder.join("Journal.crane-backup.json");
        let unrelated = folder.join("Journal.crane-backup.tmp");
        fs::write(&unrelated, b"keep me").expect("sibling fixture should be written");

        write_atomic_bytes(&path, b"backup").expect("atomic write should succeed");

        assert_eq!(
            fs::read(&path).expect("destination should exist"),
            b"backup"
        );
        assert_eq!(
            fs::read(&unrelated).expect("unrelated sibling should remain"),
            b"keep me"
        );
        let _ = fs::remove_dir_all(folder);
    }
}
