use crate::{read_limited, write_atomic_bytes, StorageError};
use crane_domain::{validate_store, Store};
use std::fs::File;
use std::path::PathBuf;

const MAX_STORE_BYTES: usize = 256 * 1024 * 1024;

pub struct JsonRepository {
    path: PathBuf,
}

impl JsonRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_or_seed(&mut self, now: u64) -> Result<Store, StorageError> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let seed = Store::seed(now);
                self.commit(&seed)?;
                return Ok(seed);
            }
            Err(error) => {
                return Err(StorageError::new(format!(
                    "Could not read the local store: {error}"
                )));
            }
        };
        let bytes = read_limited(file, MAX_STORE_BYTES)?;
        let store: Store = serde_json::from_slice(&bytes).map_err(|error| {
            StorageError::new(format!("The local store is not valid JSON: {error}"))
        })?;
        validate_store(&store).map_err(|error| StorageError::new(error.to_string()))?;
        Ok(store)
    }

    pub fn commit(&mut self, candidate: &Store) -> Result<(), StorageError> {
        validate_store(candidate).map_err(|error| StorageError::new(error.to_string()))?;
        let bytes = serde_json::to_vec_pretty(candidate)
            .map_err(|error| StorageError::new(format!("Could not serialize notes: {error}")))?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(StorageError::new(
                "The local store is larger than the 256 MiB limit.",
            ));
        }
        write_atomic_bytes(&self.path, &bytes)
    }
}
