use crate::{validate_destructive_path, PlatformError};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyProcessStatus {
    Absent,
    Running,
    Unknown,
}

pub fn classify_legacy_process_paths(
    current_executable: &Path,
    process_paths: impl IntoIterator<Item = Result<PathBuf, PlatformError>>,
) -> LegacyProcessStatus {
    let current = normalized_path(current_executable);
    let mut unknown = false;
    for process_path in process_paths {
        let path = match process_path {
            Ok(path) => path,
            Err(_) => {
                unknown = true;
                continue;
            }
        };
        let is_crane = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("crane.exe"));
        if is_crane && normalized_path(&path) != current {
            return LegacyProcessStatus::Running;
        }
    }
    if unknown {
        LegacyProcessStatus::Unknown
    } else {
        LegacyProcessStatus::Absent
    }
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}

#[cfg(windows)]
pub fn legacy_process_status() -> Result<LegacyProcessStatus, PlatformError> {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let current_executable = std::env::current_exe().map_err(|error| {
        PlatformError::new(format!("Could not resolve the current executable: {error}"))
    })?;
    let current_pid = std::process::id();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(PlatformError::new(format!(
            "Could not enumerate running processes: {}",
            std::io::Error::last_os_error()
        )));
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut candidates = Vec::new();
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        let name_end = entry
            .szExeFile
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..name_end]);
        if entry.th32ProcessID != current_pid && name.eq_ignore_ascii_case("crane.exe") {
            let process =
                unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID) };
            if process.is_null() {
                candidates.push(Err(PlatformError::new(format!(
                    "Could not inspect Crane process {}: {}",
                    entry.th32ProcessID,
                    std::io::Error::last_os_error()
                ))));
            } else {
                let mut buffer = vec![0u16; 32_768];
                let mut length = buffer.len() as u32;
                let queried = unsafe {
                    QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length)
                };
                unsafe {
                    CloseHandle(process);
                }
                if queried == 0 {
                    candidates.push(Err(PlatformError::new(format!(
                        "Could not resolve Crane process {}: {}",
                        entry.th32ProcessID,
                        std::io::Error::last_os_error()
                    ))));
                } else {
                    buffer.truncate(length as usize);
                    candidates.push(Ok(PathBuf::from(String::from_utf16_lossy(&buffer))));
                }
            }
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe {
        CloseHandle(snapshot);
    }

    Ok(classify_legacy_process_paths(
        &current_executable,
        candidates,
    ))
}

#[cfg(not(windows))]
pub fn legacy_process_status() -> Result<LegacyProcessStatus, PlatformError> {
    Ok(LegacyProcessStatus::Absent)
}

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
