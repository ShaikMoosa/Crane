mod atomic_replace;
mod environment;
mod instance;
mod paths;

use std::fmt::{Display, Formatter};

pub use atomic_replace::{replace_atomic, validate_destructive_path};
pub use environment::SystemEnvironment;
pub use instance::LibraryGuard;
pub use paths::resolve_legacy_data_dir;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformError {
    message: String,
}

impl PlatformError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for PlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PlatformError {}
