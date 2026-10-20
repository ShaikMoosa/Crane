mod application_adapter;
mod atomic;
mod bounded;
mod repository;
mod settings;
mod transfers;

use std::fmt::{Display, Formatter};

pub use atomic::{write_atomic_bytes, write_atomic_bytes_with};
pub use bounded::read_limited;
pub use repository::JsonRepository;
pub use settings::{
    load_settings, save_settings, save_settings_with, NativeSettings, ThemeChoice,
    SETTINGS_SCHEMA_VERSION,
};
pub use transfers::{write_backup_with_limit, write_library_export_with, FilesystemTransfers};

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
