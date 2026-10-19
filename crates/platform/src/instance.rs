use crate::{validate_destructive_path, PlatformError};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct LibraryGuard {
    file: Option<File>,
    path: PathBuf,
}

impl LibraryGuard {
    pub fn acquire(root: &Path) -> Result<Self, PlatformError> {
        validate_destructive_path(root)?;
        fs::create_dir_all(root).map_err(|error| {
            PlatformError::new(format!("Could not create the library directory: {error}"))
        })?;
        validate_destructive_path(root)?;
        let path = root.join(".crane-library.lock");
        let file = open_exclusive(&path)?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }
}

impl Drop for LibraryGuard {
    fn drop(&mut self) {
        self.file.take();
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
fn open_exclusive(path: &Path) -> Result<File, PlatformError> {
    use std::os::windows::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(path)
        .map_err(|error| {
            if error.raw_os_error() == Some(32) {
                PlatformError::new("The Crane library is already in use.")
            } else {
                PlatformError::new(format!("Could not acquire the library guard: {error}"))
            }
        })
}

#[cfg(not(windows))]
fn open_exclusive(path: &Path) -> Result<File, PlatformError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                PlatformError::new("The Crane library is already in use.")
            } else {
                PlatformError::new(format!("Could not acquire the library guard: {error}"))
            }
        })
}
