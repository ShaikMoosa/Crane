use crate::{write_atomic_bytes_with, StorageError};
use crane_platform::PlatformError;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Light,
    #[default]
    Dark,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSettings {
    pub schema_version: u32,
    pub theme: ThemeChoice,
    pub reduced_motion: bool,
}

impl Default for NativeSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            theme: ThemeChoice::Dark,
            reduced_motion: false,
        }
    }
}

pub fn load_settings(path: &Path) -> Result<NativeSettings, StorageError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(NativeSettings::default());
        }
        Err(error) => {
            return Err(StorageError::new(format!(
                "Could not read native settings: {error}"
            )));
        }
    };
    let settings: NativeSettings = serde_json::from_slice(&bytes)
        .map_err(|error| StorageError::new(format!("Could not parse native settings: {error}")))?;
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(StorageError::new(format!(
            "Unsupported settings version {}.",
            settings.schema_version
        )));
    }
    Ok(settings)
}

pub fn save_settings(path: &Path, settings: &NativeSettings) -> Result<(), StorageError> {
    save_settings_with(path, settings, crane_platform::replace_atomic)
}

#[doc(hidden)]
pub fn save_settings_with(
    path: &Path,
    settings: &NativeSettings,
    replace: impl FnOnce(&Path, &Path) -> Result<(), PlatformError>,
) -> Result<(), StorageError> {
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(StorageError::new(format!(
            "Unsupported settings version {}.",
            settings.schema_version
        )));
    }
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| StorageError::new(format!("Could not encode native settings: {error}")))?;
    write_atomic_bytes_with(path, &bytes, replace)
}
