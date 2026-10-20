use crate::PlatformError;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogSelection<T> {
    Selected(T),
    Cancelled,
}

pub trait DialogProvider: Send {
    fn pick_markdown(&mut self) -> Result<DialogSelection<Vec<PathBuf>>, PlatformError>;
    fn save_note(
        &mut self,
        suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError>;
    fn pick_export_folder(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError>;
    fn save_backup(
        &mut self,
        suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError>;
    fn confirm_replace(&mut self, path: &Path) -> Result<bool, PlatformError>;
    fn pick_backup(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError>;
}

#[derive(Default)]
pub struct NativeDialogs;

#[cfg(windows)]
impl DialogProvider for NativeDialogs {
    fn pick_markdown(&mut self) -> Result<DialogSelection<Vec<PathBuf>>, PlatformError> {
        Ok(rfd::FileDialog::new()
            .set_title("Import Markdown")
            .add_filter("Markdown", &["md"])
            .pick_files()
            .filter(|paths| !paths.is_empty())
            .map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn save_note(
        &mut self,
        suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(rfd::FileDialog::new()
            .set_title("Export Note")
            .set_file_name(suggested_name)
            .add_filter("Markdown", &["md"])
            .save_file()
            .map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn pick_export_folder(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(rfd::FileDialog::new()
            .set_title("Export Crane Library")
            .pick_folder()
            .map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn save_backup(
        &mut self,
        suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(rfd::FileDialog::new()
            .set_title("Create Crane Backup")
            .set_file_name(suggested_name)
            .add_filter("Crane backup", &["json"])
            .save_file()
            .map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }

    fn confirm_replace(&mut self, path: &Path) -> Result<bool, PlatformError> {
        let name = path
            .file_name()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        let result = rfd::MessageDialog::new()
            .set_title("Replace existing file?")
            .set_description(format!("A file named {name} already exists. Replace it?"))
            .set_buttons(rfd::MessageButtons::OkCancel)
            .show();
        Ok(matches!(result, rfd::MessageDialogResult::Ok))
    }

    fn pick_backup(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Ok(rfd::FileDialog::new()
            .set_title("Restore Crane Backup")
            .add_filter("Crane backup", &["json"])
            .pick_file()
            .map_or(DialogSelection::Cancelled, DialogSelection::Selected))
    }
}

#[cfg(not(windows))]
impl DialogProvider for NativeDialogs {
    fn pick_markdown(&mut self) -> Result<DialogSelection<Vec<PathBuf>>, PlatformError> {
        Err(unavailable())
    }

    fn save_note(
        &mut self,
        _suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Err(unavailable())
    }

    fn pick_export_folder(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Err(unavailable())
    }

    fn save_backup(
        &mut self,
        _suggested_name: &str,
    ) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Err(unavailable())
    }

    fn confirm_replace(&mut self, _path: &Path) -> Result<bool, PlatformError> {
        Err(unavailable())
    }

    fn pick_backup(&mut self) -> Result<DialogSelection<PathBuf>, PlatformError> {
        Err(unavailable())
    }
}

#[cfg(not(windows))]
fn unavailable() -> PlatformError {
    PlatformError::new("Native dialogs are not available on this platform build.")
}
