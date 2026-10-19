use crate::PlatformError;
use std::path::PathBuf;

pub fn resolve_legacy_data_dir() -> Result<PathBuf, PlatformError> {
    dirs::data_dir()
        .map(|root| root.join("app.crane.desktop"))
        .ok_or_else(|| PlatformError::new("Could not find the roaming application-data directory."))
}
