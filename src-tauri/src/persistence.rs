use crate::model::Store;
use crane_storage::JsonRepository;
use std::path::Path;

pub fn load_or_seed(path: &Path, timestamp: u64) -> Result<Store, String> {
    JsonRepository::new(path.to_path_buf())
        .load_or_seed(timestamp)
        .map_err(|error| error.to_string())
}

pub fn save(path: &Path, store: &Store) -> Result<(), String> {
    JsonRepository::new(path.to_path_buf())
        .commit(store)
        .map_err(|error| error.to_string())
}

pub(crate) fn write_atomic_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    crane_storage::write_atomic_bytes(path, bytes).map_err(|error| error.to_string())
}
