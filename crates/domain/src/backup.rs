use crate::{validate_store, Store, ValidationCode, ValidationError, MAX_RENDERABLE_TIMESTAMP};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const BACKUP_FORMAT: &str = "crane-backup";
pub const BACKUP_VERSION: u32 = 1;
pub const MAX_BACKUP_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEnvelope {
    pub format: String,
    pub backup_version: u32,
    pub created_at: u64,
    pub app_version: String,
    pub store: Store,
    #[serde(default, flatten)]
    pub extensions: Map<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupInspection {
    pub created_at: u64,
    pub total: usize,
    pub active: usize,
    pub archived: usize,
    pub trashed: usize,
}

pub fn make_backup(
    store: &Store,
    created_at: u64,
    app_version: &str,
) -> Result<BackupEnvelope, ValidationError> {
    validate_store(store)?;
    validate_backup_timestamp(created_at)?;
    Ok(BackupEnvelope {
        format: BACKUP_FORMAT.into(),
        backup_version: BACKUP_VERSION,
        created_at,
        app_version: app_version.into(),
        store: store.clone(),
        extensions: Map::new(),
    })
}

pub fn serialize_backup(envelope: &BackupEnvelope) -> Result<Vec<u8>, ValidationError> {
    validate_envelope(envelope)?;
    let bytes = serde_json::to_vec_pretty(envelope).map_err(|error| {
        ValidationError::new(
            ValidationCode::InvalidBackup,
            format!("Could not serialize the backup: {error}"),
        )
    })?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(backup_size_error());
    }
    Ok(bytes)
}

pub fn inspect_backup(bytes: &[u8]) -> Result<(BackupEnvelope, BackupInspection), ValidationError> {
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(backup_size_error());
    }
    let envelope: BackupEnvelope = serde_json::from_slice(bytes).map_err(|error| {
        ValidationError::new(
            ValidationCode::InvalidBackup,
            format!("The backup is not valid JSON: {error}"),
        )
    })?;
    validate_envelope(&envelope)?;
    let inspection = BackupInspection {
        created_at: envelope.created_at,
        total: envelope.store.notes.len(),
        active: envelope
            .store
            .notes
            .iter()
            .filter(|note| !note.archived && !note.trashed)
            .count(),
        archived: envelope
            .store
            .notes
            .iter()
            .filter(|note| note.archived && !note.trashed)
            .count(),
        trashed: envelope
            .store
            .notes
            .iter()
            .filter(|note| note.trashed)
            .count(),
    };
    Ok((envelope, inspection))
}

fn validate_envelope(envelope: &BackupEnvelope) -> Result<(), ValidationError> {
    if envelope.format != BACKUP_FORMAT {
        return Err(ValidationError::new(
            ValidationCode::BackupFormat,
            "The selected file is not a Crane backup.",
        ));
    }
    if envelope.backup_version != BACKUP_VERSION {
        return Err(ValidationError::new(
            ValidationCode::UnsupportedBackupVersion,
            format!("Unsupported backup version {}.", envelope.backup_version),
        ));
    }
    validate_backup_timestamp(envelope.created_at)?;
    validate_store(&envelope.store)
}

fn validate_backup_timestamp(created_at: u64) -> Result<(), ValidationError> {
    if created_at > MAX_RENDERABLE_TIMESTAMP {
        return Err(ValidationError::new(
            ValidationCode::TimestampOutOfRange,
            "The backup timestamp is outside the supported date range.",
        ));
    }
    Ok(())
}

fn backup_size_error() -> ValidationError {
    ValidationError::new(
        ValidationCode::BackupTooLarge,
        "The backup exceeds the 256 MB limit.",
    )
}
