use crane_storage::{save_settings, NativeSettings};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SettingsController {
    value: NativeSettings,
    path: Option<PathBuf>,
}

impl SettingsController {
    pub fn new(value: NativeSettings, path: Option<PathBuf>) -> Self {
        Self { value, path }
    }

    pub fn value(&self) -> &NativeSettings {
        &self.value
    }

    pub fn commit(&mut self, candidate: NativeSettings) -> Result<(), String> {
        let Some(path) = &self.path else {
            self.value = candidate;
            return Ok(());
        };
        save_settings(path, &candidate).map_err(|error| error.to_string())?;
        self.value = candidate;
        Ok(())
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}
