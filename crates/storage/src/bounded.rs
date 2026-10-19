use crate::StorageError;
use std::io::Read;

pub fn read_limited(mut reader: impl Read, limit: usize) -> Result<Vec<u8>, StorageError> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    reader
        .by_ref()
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| StorageError::new(format!("Could not read local data: {error}")))?;
    if bytes.len() > limit {
        return Err(StorageError::new(format!(
            "The local data is larger than the {limit}-byte limit."
        )));
    }
    Ok(bytes)
}
