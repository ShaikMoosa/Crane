use crate::{read_limited, write_atomic_bytes, JsonRepository};
use crane_application::{AppError, Environment, ErrorCode, TransferIo};
use crane_platform::{legacy_process_status, LegacyProcessStatus};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Component, Path};

const RECEIPT_FILE_NAME: &str = "native-migration-v1.json";
const RECEIPT_VERSION: u32 = 1;
const MAX_RECEIPT_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutoverReport {
    pub note_count: usize,
    pub backup_file_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CutoverReceipt {
    receipt_version: u32,
    note_count: usize,
    backup_file_name: String,
}

pub fn prepare_cutover(
    repository: &mut JsonRepository,
    transfers: &mut dyn TransferIo,
    environment: &mut dyn Environment,
) -> Result<CutoverReport, AppError> {
    let status = legacy_process_status().map_err(platform)?;
    prepare_cutover_with_status(repository, transfers, environment, status)
}

pub fn prepare_cutover_with_status(
    repository: &mut JsonRepository,
    transfers: &mut dyn TransferIo,
    environment: &mut dyn Environment,
    status: LegacyProcessStatus,
) -> Result<CutoverReport, AppError> {
    if status != LegacyProcessStatus::Absent {
        return Err(AppError::new(
            ErrorCode::Busy,
            match status {
                LegacyProcessStatus::Running => {
                    "Close the installed legacy Crane application before starting native Crane."
                }
                LegacyProcessStatus::Unknown => {
                    "Crane could not verify that the legacy application is closed."
                }
                LegacyProcessStatus::Absent => unreachable!(),
            },
        ));
    }

    let store = repository.load_existing().map_err(storage)?;
    let root = repository
        .path()
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::Io, "The Crane data path has no parent folder."))?;
    let receipt_path = root.join(RECEIPT_FILE_NAME);
    if let Some(receipt) = read_receipt(&receipt_path)? {
        return Ok(CutoverReport {
            note_count: receipt.note_count,
            backup_file_name: receipt.backup_file_name,
        });
    }

    let (note_count, backup_file_name) = match store {
        Some(store) => (
            store.notes.len(),
            transfers.write_pre_migration_backup(&store, environment.now_millis())?,
        ),
        None => (0, String::new()),
    };
    validate_backup_file_name(note_count, &backup_file_name)?;
    let receipt = CutoverReceipt {
        receipt_version: RECEIPT_VERSION,
        note_count,
        backup_file_name: backup_file_name.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(|error| {
        AppError::new(
            ErrorCode::Io,
            format!("Could not serialize the migration receipt: {error}"),
        )
    })?;
    write_atomic_bytes(&receipt_path, &bytes).map_err(storage)?;
    Ok(CutoverReport {
        note_count,
        backup_file_name,
    })
}

fn read_receipt(path: &Path) -> Result<Option<CutoverReceipt>, AppError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Ok(None),
    };
    let Ok(bytes) = read_limited(file, MAX_RECEIPT_BYTES) else {
        return Ok(None);
    };
    let Ok(receipt) = serde_json::from_slice::<CutoverReceipt>(&bytes) else {
        return Ok(None);
    };
    if receipt.receipt_version != RECEIPT_VERSION
        || validate_backup_file_name(receipt.note_count, &receipt.backup_file_name).is_err()
    {
        return Ok(None);
    }
    Ok(Some(receipt))
}

fn validate_backup_file_name(note_count: usize, value: &str) -> Result<(), AppError> {
    if note_count == 0 && value.is_empty() {
        return Ok(());
    }
    let mut components = Path::new(value).components();
    if !value.ends_with(".crane-backup.json")
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(AppError::new(
            ErrorCode::Io,
            "The pre-migration backup filename is invalid.",
        ));
    }
    Ok(())
}

fn storage(error: crate::StorageError) -> AppError {
    AppError::new(ErrorCode::Io, error.to_string())
}

fn platform(error: crane_platform::PlatformError) -> AppError {
    AppError::new(ErrorCode::Io, error.to_string())
}
