use crate::PlatformError;
use std::fs;
use std::path::Path;

pub fn validate_destructive_path(path: &Path) -> Result<(), PlatformError> {
    for candidate in path.ancestors() {
        match fs::symlink_metadata(candidate) {
            Ok(metadata) if is_reparse_point(&metadata) => {
                return Err(PlatformError::new(format!(
                    "Refusing a destructive write through reparse point {}.",
                    candidate.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(PlatformError::new(format!(
                    "Could not inspect the write destination safely: {error}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

pub fn replace_atomic(temporary: &Path, destination: &Path) -> Result<(), PlatformError> {
    validate_destructive_path(temporary)?;
    validate_destructive_path(destination)?;

    let temporary_parent = temporary
        .parent()
        .ok_or_else(|| PlatformError::new("The temporary path has no parent directory."))?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| PlatformError::new("The destination path has no parent directory."))?;
    let temporary_parent = fs::canonicalize(temporary_parent).map_err(|error| {
        PlatformError::new(format!(
            "Could not resolve the temporary directory: {error}"
        ))
    })?;
    let destination_parent = fs::canonicalize(destination_parent).map_err(|error| {
        PlatformError::new(format!(
            "Could not resolve the destination directory: {error}"
        ))
    })?;
    if temporary_parent != destination_parent {
        return Err(PlatformError::new(
            "Atomic replacement requires a temporary file beside its destination.",
        ));
    }

    replace_file(temporary, destination)
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), PlatformError> {
    fs::rename(temporary, destination)
        .map_err(|error| PlatformError::new(format!("Could not replace the local store: {error}")))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), PlatformError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let replaced = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        return Err(PlatformError::new(format!(
            "Could not replace the local store: {}",
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}
