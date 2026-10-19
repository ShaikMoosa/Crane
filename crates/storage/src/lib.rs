mod atomic;
mod bounded;
mod repository;

use std::fmt::{Display, Formatter};

pub use atomic::{write_atomic_bytes, write_atomic_bytes_with};
pub use bounded::read_limited;
pub use repository::JsonRepository;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageError {
    message: String,
}

impl StorageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for StorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StorageError {}

impl From<crane_platform::PlatformError> for StorageError {
    fn from(error: crane_platform::PlatformError) -> Self {
        Self::new(error.to_string())
    }
}
