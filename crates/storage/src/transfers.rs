use crate::{read_limited, write_atomic_bytes, StorageError};
use crane_application::{AppError, ErrorCode, ExportSummary, Outcome, TransferIo};
use crane_domain::{
    build_export_manifest, make_backup, serialize_backup_with_limit, serialize_markdown,
    suggested_markdown_filename, BackupEnvelope, ExportManifest, NamedBytes, Note, Store,
    MAX_BACKUP_BYTES, MAX_BATCH_BYTES,
};
use crane_platform::{DialogProvider, DialogSelection};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TRANSFER_COUNTER: AtomicU64 = AtomicU64::new(0);
const BACKUP_SUFFIX: &str = ".crane-backup.json";

pub struct FilesystemTransfers {
    dialogs: Box<dyn DialogProvider>,
    safety_directory: PathBuf,
    app_version: String,
}

impl FilesystemTransfers {
    pub fn new(
        dialogs: Box<dyn DialogProvider>,
        safety_directory: PathBuf,
        app_version: impl Into<String>,
    ) -> Self {
        Self {
            dialogs,
            safety_directory,
            app_version: app_version.into(),
        }
    }
}

impl TransferIo for FilesystemTransfers {
    fn pick_markdown(&mut self) -> Result<Outcome<Vec<NamedBytes>>, AppError> {
        let DialogSelection::Selected(paths) = self.dialogs.pick_markdown().map_err(platform)?
        else {
            return Ok(Outcome::Cancelled);
        };
        let mut remaining = MAX_BATCH_BYTES;
        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::Validation,
                        "A selected Markdown filename is not valid Unicode.",
                    )
                })?
                .to_string();
            let file = File::open(&path).map_err(|error| {
                AppError::new(
                    ErrorCode::Io,
                    format!("Could not read {}: {error}", path.display()),
                )
            })?;
            let bytes = read_limited(file, remaining).map_err(storage)?;
            remaining = remaining.saturating_sub(bytes.len());
            files.push(NamedBytes { name, bytes });
        }
        Ok(Outcome::Completed(files))
    }

    fn export_note(&mut self, note: &Note) -> Result<Outcome<usize>, AppError> {
        let suggested = suggested_markdown_filename(note);
        let DialogSelection::Selected(selected) =
            self.dialogs.save_note(&suggested).map_err(platform)?
        else {
            return Ok(Outcome::Cancelled);
        };
        let path = ensure_suffix(selected, ".md");
        write_atomic_bytes(&path, serialize_markdown(note).as_bytes()).map_err(storage)?;
        Ok(Outcome::Completed(1))
    }

    fn export_library(&mut self, store: &Store) -> Result<Outcome<ExportSummary>, AppError> {
        let DialogSelection::Selected(destination) =
            self.dialogs.pick_export_folder().map_err(platform)?
        else {
            return Ok(Outcome::Cancelled);
        };
        let folder_name = format!(
            "Crane Export {}",
            format_export_timestamp(system_time_millis())
        );
        let manifest = build_export_manifest(store)?;
        let summary =
            write_library_export_with(&destination, &manifest, &folder_name, write_new_file)
                .map_err(storage)?;
        Ok(Outcome::Completed(summary))
    }

    fn create_backup(
        &mut self,
        store: &Store,
        created_at: u64,
    ) -> Result<Outcome<usize>, AppError> {
        let suggested = format!(
            "Crane Backup {}{BACKUP_SUFFIX}",
            format_export_timestamp(created_at)
        );
        let DialogSelection::Selected(selected) =
            self.dialogs.save_backup(&suggested).map_err(platform)?
        else {
            return Ok(Outcome::Cancelled);
        };
        let path = normalize_backup_path(selected);
        if path.exists() && !self.dialogs.confirm_replace(&path).map_err(platform)? {
            return Ok(Outcome::Cancelled);
        }
        let envelope = make_backup(store, created_at, &self.app_version)?;
        write_backup_with_limit(&path, &envelope, MAX_BACKUP_BYTES).map_err(storage)?;
        Ok(Outcome::Completed(store.notes.len()))
    }

    fn pick_backup(&mut self) -> Result<Outcome<Vec<u8>>, AppError> {
        let DialogSelection::Selected(path) = self.dialogs.pick_backup().map_err(platform)? else {
            return Ok(Outcome::Cancelled);
        };
        let file = File::open(&path).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not read {}: {error}", path.display()),
            )
        })?;
        Ok(Outcome::Completed(
            read_limited(file, MAX_BACKUP_BYTES).map_err(storage)?,
        ))
    }

    fn write_safety_backup(&mut self, store: &Store, created_at: u64) -> Result<(), AppError> {
        fs::create_dir_all(&self.safety_directory).map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                format!("Could not create the backup directory: {error}"),
            )
        })?;
        let envelope = make_backup(store, created_at, &self.app_version)?;
        let path = reserve_safety_path(&self.safety_directory, created_at).map_err(storage)?;
        let result = write_backup_with_limit(&path, &envelope, MAX_BACKUP_BYTES);
        if result.is_err() {
            let _ = fs::remove_file(&path);
        }
        result.map_err(storage)
    }
}

pub fn write_backup_with_limit(
    path: &Path,
    envelope: &BackupEnvelope,
    byte_limit: usize,
) -> Result<(), StorageError> {
    let bytes = serialize_backup_with_limit(envelope, byte_limit)
        .map_err(|error| StorageError::new(error.to_string()))?;
    write_atomic_bytes(path, &bytes)
}

pub fn write_library_export_with(
    destination: &Path,
    manifest: &ExportManifest,
    folder_name: &str,
    mut writer: impl FnMut(&Path, &[u8]) -> io::Result<()>,
) -> Result<ExportSummary, StorageError> {
    ensure_single_component(folder_name)?;
    fs::create_dir_all(destination).map_err(|error| {
        StorageError::new(format!("Could not open the export destination: {error}"))
    })?;
    let final_path = destination.join(folder_name);
    if final_path.exists() {
        return Err(StorageError::new(format!(
            "An export named {folder_name} already exists."
        )));
    }
    let reservation = destination.join(format!(".{folder_name}.crane-reserved"));
    let reservation_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&reservation)
        .map_err(|error| {
            StorageError::new(format!("Could not reserve the export name: {error}"))
        })?;
    drop(reservation_file);
    let temporary = create_temporary_directory(destination)?;
    let result = (|| {
        for entry in &manifest.entries {
            let relative = safe_relative_path(&entry.relative_path)?;
            let path = temporary.join(relative);
            let parent = path
                .parent()
                .ok_or_else(|| StorageError::new("An export entry has no parent directory."))?;
            fs::create_dir_all(parent).map_err(|error| {
                StorageError::new(format!("Could not create an export folder: {error}"))
            })?;
            writer(&path, &entry.contents).map_err(|error| {
                StorageError::new(format!("Could not write an exported note: {error}"))
            })?;
        }
        fs::rename(&temporary, &final_path).map_err(|error| {
            StorageError::new(format!("Could not publish the library export: {error}"))
        })?;
        Ok(ExportSummary {
            exported_count: manifest.entries.len(),
            folder_name: folder_name.to_string(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    let _ = fs::remove_file(&reservation);
    result
}

fn reserve_safety_path(directory: &Path, timestamp: u64) -> Result<PathBuf, StorageError> {
    for suffix in 1..=10_000usize {
        let name = if suffix == 1 {
            format!("pre-restore-{timestamp}{BACKUP_SUFFIX}")
        } else {
            format!("pre-restore-{timestamp}-{suffix}{BACKUP_SUFFIX}")
        };
        let path = directory.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(StorageError::new(format!(
                    "Could not reserve a safety-backup filename: {error}"
                )))
            }
        }
    }
    Err(StorageError::new(
        "Could not reserve a unique safety-backup filename.",
    ))
}

fn create_temporary_directory(destination: &Path) -> Result<PathBuf, StorageError> {
    for _ in 0..1_000 {
        let sequence = TRANSFER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = destination.join(format!(
            ".crane-export-{}-{sequence}.tmp",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(StorageError::new(format!(
                    "Could not create a temporary export folder: {error}"
                )))
            }
        }
    }
    Err(StorageError::new(
        "Could not reserve a temporary export folder.",
    ))
}

fn safe_relative_path(value: &str) -> Result<PathBuf, StorageError> {
    let path = PathBuf::from(value);
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(StorageError::new("An export entry has an unsafe path."));
    }
    Ok(path)
}

fn ensure_single_component(value: &str) -> Result<(), StorageError> {
    let mut components = Path::new(value).components();
    if value.is_empty()
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(StorageError::new("The export folder name is unsafe."));
    }
    Ok(())
}

fn write_new_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

fn normalize_backup_path(path: PathBuf) -> PathBuf {
    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(BACKUP_SUFFIX)
    {
        return path;
    }
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        let mut without_json = path;
        without_json.set_extension("");
        let mut value = without_json.into_os_string();
        value.push(BACKUP_SUFFIX);
        return PathBuf::from(value);
    }
    ensure_suffix(path, BACKUP_SUFFIX)
}

fn ensure_suffix(path: PathBuf, suffix: &str) -> PathBuf {
    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(&suffix.to_ascii_lowercase())
    {
        return path;
    }
    let mut value = path.into_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn system_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn format_export_timestamp(timestamp_millis: u64) -> String {
    let seconds = (timestamp_millis / 1_000).min(i64::MAX as u64) as i64;
    let days = seconds / 86_400;
    let seconds_in_day = seconds % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}{minute:02}{second:02}")
}

fn storage(error: StorageError) -> AppError {
    AppError::new(ErrorCode::Io, error.to_string())
}

fn platform(error: crane_platform::PlatformError) -> AppError {
    AppError::new(ErrorCode::Io, error.to_string())
}
