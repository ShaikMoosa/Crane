use crate::model::{Store, SCHEMA_VERSION};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

pub fn load_or_seed(path: &Path, timestamp: u64) -> Result<Store, String> {
    if !path.exists() {
        let store = Store::seed(timestamp);
        save(path, &store)?;
        return Ok(store);
    }
    let bytes = fs::read(path).map_err(|error| format!("Could not read the local store: {error}"))?;
    let store: Store = serde_json::from_slice(&bytes).map_err(|error| format!("The local store is not valid JSON: {error}"))?;
    if store.schema_version != SCHEMA_VERSION {
        return Err(format!("Unsupported store version {}.", store.schema_version));
    }
    Ok(store)
}

pub fn save(path: &Path, store: &Store) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "The data path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("Could not create the data directory: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(store).map_err(|error| format!("Could not serialize notes: {error}"))?;
    let mut file = File::create(&temporary).map_err(|error| format!("Could not create a temporary store: {error}"))?;
    file.write_all(&bytes).map_err(|error| format!("Could not write notes: {error}"))?;
    file.sync_all().map_err(|error| format!("Could not flush notes to disk: {error}"))?;
    replace_file(&temporary, path)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(temporary, destination).map_err(|error| format!("Could not replace the local store: {error}"))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};

    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    let replaced = unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) };
    if replaced == 0 {
        return Err(format!("Could not replace the local store: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_store() {
        let folder = std::env::temp_dir().join(format!("crane-test-{}", std::process::id()));
        let path = folder.join("notes.json");
        let source = Store::seed(42);
        save(&path, &source).expect("save should succeed");
        let loaded = load_or_seed(&path, 99).expect("load should succeed");
        assert_eq!(loaded.notes.len(), source.notes.len());
        let _ = fs::remove_dir_all(folder);
    }
}

