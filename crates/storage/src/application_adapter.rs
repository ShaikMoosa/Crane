use crate::{JsonRepository, StorageError};
use crane_application::{AppError, ErrorCode, LibraryRepository};
use crane_domain::Store;

impl LibraryRepository for JsonRepository {
    fn load_or_seed(&mut self, now: u64) -> Result<Store, AppError> {
        JsonRepository::load_or_seed(self, now).map_err(map_storage_error)
    }

    fn commit(&mut self, candidate: &Store) -> Result<(), AppError> {
        JsonRepository::commit(self, candidate).map_err(map_storage_error)
    }
}

fn map_storage_error(error: StorageError) -> AppError {
    let message = error.to_string();
    let code = if message.starts_with("Unsupported store version") {
        ErrorCode::UnsupportedVersion
    } else if message.starts_with("The store contains")
        || message.starts_with("A note")
        || message.starts_with("The note")
    {
        ErrorCode::Validation
    } else {
        ErrorCode::Io
    };
    AppError::new(code, message)
}
