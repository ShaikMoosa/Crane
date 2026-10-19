use crate::StorageError;
use crane_platform::{replace_atomic, validate_destructive_path, PlatformError};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    write_atomic_bytes_with(path, bytes, replace_atomic)
}

pub fn write_atomic_bytes_with(
    path: &Path,
    bytes: &[u8],
    replace: impl FnOnce(&Path, &Path) -> Result<(), PlatformError>,
) -> Result<(), StorageError> {
    validate_destructive_path(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| StorageError::new("The data path has no parent directory."))?;
    fs::create_dir_all(parent).map_err(|error| {
        StorageError::new(format!("Could not create the data directory: {error}"))
    })?;
    validate_destructive_path(path)?;

    let (temporary, mut file) = create_temporary_file(parent)?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::new(format!(
            "Could not write local data: {error}"
        )));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::new(format!(
            "Could not flush local data to disk: {error}"
        )));
    }
    drop(file);
    if let Err(error) = replace(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn create_temporary_file(parent: &Path) -> Result<(PathBuf, File), StorageError> {
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
                return Err(StorageError::new(format!(
                    "Could not create a temporary file: {error}"
                )));
            }
        }
    }
    Err(StorageError::new(
        "Could not reserve a unique temporary file.",
    ))
}
