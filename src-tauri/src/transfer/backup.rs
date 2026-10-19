use super::TransferError;
use crate::model::{validate_store, Store, MAX_RENDERABLE_TIMESTAMP, SCHEMA_VERSION};
use crate::persistence::write_atomic_bytes;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const BACKUP_FORMAT: &str = "crane-backup";
const BACKUP_VERSION: u32 = 1;
pub(crate) const MAX_BACKUP_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEnvelope {
    pub format: String,
    pub backup_version: u32,
    pub created_at: u64,
    pub app_version: String,
    pub store: Store,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupInspection {
    pub created_at: u64,
    pub total: usize,
    pub active: usize,
    pub archived: usize,
    pub trashed: usize,
}

#[derive(Clone, Debug)]
pub struct PendingRestore {
    pub token: String,
    pub inspection: BackupInspection,
    pub replacement: Store,
}

pub fn serialize_backup(
    store: &Store,
    created_at: u64,
    app_version: &str,
) -> Result<Vec<u8>, TransferError> {
    let envelope = make_envelope(store, created_at, app_version)?;
    serialize_envelope(&envelope)
}

pub fn make_envelope(
    store: &Store,
    created_at: u64,
    app_version: &str,
) -> Result<BackupEnvelope, TransferError> {
    validate_store(store).map_err(TransferError::Validation)?;
    validate_backup_timestamp(created_at)?;
    Ok(BackupEnvelope {
        format: BACKUP_FORMAT.into(),
        backup_version: BACKUP_VERSION,
        created_at,
        app_version: app_version.into(),
        store: store.clone(),
    })
}

fn serialize_envelope(envelope: &BackupEnvelope) -> Result<Vec<u8>, TransferError> {
    serialize_envelope_with_limit(envelope, MAX_BACKUP_BYTES)
}

fn serialize_envelope_with_limit(
    envelope: &BackupEnvelope,
    byte_limit: usize,
) -> Result<Vec<u8>, TransferError> {
    let bytes = serde_json::to_vec_pretty(envelope)
        .map_err(|error| TransferError::Json(format!("Could not serialize the backup: {error}")))?;
    if bytes.len() > byte_limit {
        return Err(TransferError::BackupSize);
    }
    Ok(bytes)
}

pub fn inspect_backup(bytes: &[u8], token: String) -> Result<PendingRestore, TransferError> {
    inspect_backup_with_limit(bytes, token, MAX_BACKUP_BYTES)
}

fn inspect_backup_with_limit(
    bytes: &[u8],
    token: String,
    byte_limit: usize,
) -> Result<PendingRestore, TransferError> {
    if bytes.len() > byte_limit {
        return Err(TransferError::BackupSize);
    }
    let envelope: BackupEnvelope = serde_json::from_slice(bytes)
        .map_err(|error| TransferError::Json(format!("The backup is not valid JSON: {error}")))?;
    if envelope.format != BACKUP_FORMAT {
        return Err(TransferError::BackupFormat);
    }
    if envelope.backup_version != BACKUP_VERSION {
        return Err(TransferError::BackupVersion(envelope.backup_version));
    }
    if envelope.store.schema_version != SCHEMA_VERSION {
        return Err(TransferError::StoreVersion(envelope.store.schema_version));
    }
    validate_backup_timestamp(envelope.created_at)?;
    validate_store(&envelope.store).map_err(TransferError::Validation)?;

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
    Ok(PendingRestore {
        token,
        inspection,
        replacement: envelope.store,
    })
}

fn validate_backup_timestamp(created_at: u64) -> Result<(), TransferError> {
    if created_at > MAX_RENDERABLE_TIMESTAMP {
        return Err(TransferError::Validation(
            "The backup timestamp is outside the supported date range.".into(),
        ));
    }
    Ok(())
}

pub fn restore_transaction(
    current: &Store,
    replacement: Store,
    write_safety: impl FnOnce(&Store) -> Result<(), TransferError>,
    write_replacement: impl FnOnce(&Store) -> Result<(), TransferError>,
) -> Result<Store, TransferError> {
    validate_store(current).map_err(TransferError::Validation)?;
    validate_store(&replacement).map_err(TransferError::Validation)?;
    write_safety(current)?;
    write_replacement(&replacement)?;
    Ok(replacement)
}

pub fn write_backup(path: &Path, envelope: &BackupEnvelope) -> Result<(), TransferError> {
    if envelope.format != BACKUP_FORMAT {
        return Err(TransferError::BackupFormat);
    }
    if envelope.backup_version != BACKUP_VERSION {
        return Err(TransferError::BackupVersion(envelope.backup_version));
    }
    validate_store(&envelope.store).map_err(TransferError::Validation)?;
    let bytes = serialize_backup(&envelope.store, envelope.created_at, &envelope.app_version)?;
    write_atomic_bytes(path, &bytes).map_err(TransferError::Io)
}

pub fn write_safety_backup(
    backup_directory: &Path,
    store: &Store,
    created_at: u64,
    app_version: &str,
) -> Result<PathBuf, TransferError> {
    let envelope = make_envelope(store, created_at, app_version)?;
    let path = unique_safety_path(backup_directory, created_at)?;
    write_backup(&path, &envelope)?;
    Ok(path)
}

fn unique_safety_path(directory: &Path, timestamp: u64) -> Result<PathBuf, TransferError> {
    for suffix in 1..=10_000usize {
        let name = if suffix == 1 {
            format!("pre-restore-{timestamp}.crane-backup.json")
        } else {
            format!("pre-restore-{timestamp}-{suffix}.crane-backup.json")
        };
        let path = directory.join(name);
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(TransferError::Io(
        "Could not reserve a unique safety-backup filename.".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Store, SCHEMA_VERSION};
    use std::cell::{Cell, RefCell};

    fn store_with_all_states() -> Store {
        let mut store = Store::seed(100);
        store.notes[1].archived = true;
        store.notes[2].trashed = true;
        store
    }

    #[test]
    fn backup_round_trip_preserves_all_note_states() {
        let store = store_with_all_states();
        let bytes = serialize_backup(&store, 500, "0.1.0").expect("backup should serialize");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("backup should be JSON");
        assert_eq!(json["format"], "crane-backup");
        assert_eq!(json["backupVersion"], 1);

        let pending = inspect_backup(&bytes, "one-use-token".into())
            .expect("serialized backup should inspect");
        assert_eq!(pending.token, "one-use-token");
        assert_eq!(pending.inspection.created_at, 500);
        assert_eq!(pending.inspection.total, 3);
        assert_eq!(pending.inspection.active, 1);
        assert_eq!(pending.inspection.archived, 1);
        assert_eq!(pending.inspection.trashed, 1);
        assert!(pending.replacement.notes[1].archived);
        assert!(pending.replacement.notes[2].trashed);
    }

    #[test]
    fn rejects_wrong_format_and_future_versions() {
        let malformed =
            inspect_backup(b"not JSON", "token".into()).expect_err("malformed JSON must fail");
        assert!(matches!(malformed, TransferError::Json(_)));

        let wrong_format = backup_json("another-app", 1, SCHEMA_VERSION);
        let error =
            inspect_backup(&wrong_format, "token".into()).expect_err("wrong formats must fail");
        assert!(matches!(error, TransferError::BackupFormat));

        let future_backup = backup_json("crane-backup", 2, SCHEMA_VERSION);
        let error = inspect_backup(&future_backup, "token".into())
            .expect_err("future backup versions must fail");
        assert!(matches!(error, TransferError::BackupVersion(2)));

        let future_store = backup_json("crane-backup", 1, SCHEMA_VERSION + 1);
        let error = inspect_backup(&future_store, "token".into())
            .expect_err("future store versions must fail");
        assert!(matches!(error, TransferError::StoreVersion(_)));
    }

    #[test]
    fn rejects_duplicate_ids_and_invalid_notes() {
        let mut duplicate = Store::seed(10);
        duplicate.notes[1].id = duplicate.notes[0].id.clone();
        let bytes = unchecked_backup_json(&duplicate);
        let error =
            inspect_backup(&bytes, "token".into()).expect_err("duplicate IDs must fail validation");
        assert!(matches!(error, TransferError::Validation(_)));

        let mut invalid = Store::seed(10);
        invalid.notes[0].id.clear();
        let bytes = unchecked_backup_json(&invalid);
        let error =
            inspect_backup(&bytes, "token".into()).expect_err("invalid notes must fail validation");
        assert!(matches!(error, TransferError::Validation(_)));
    }

    #[test]
    fn rejects_backup_over_256_mib_before_json_parse() {
        assert_eq!(MAX_BACKUP_BYTES, 256 * 1024 * 1024);
        let error = inspect_backup_with_limit(b"not JSON", "token".into(), 4)
            .expect_err("size must be checked before JSON parsing");
        assert!(matches!(error, TransferError::BackupSize));
    }

    #[test]
    fn refuses_to_create_a_backup_larger_than_the_restore_limit() {
        let envelope =
            make_envelope(&Store::seed(10), 20, "0.1.0").expect("fixture should validate");
        let encoded = serialize_envelope(&envelope).expect("fixture should serialize");

        let error = serialize_envelope_with_limit(&envelope, encoded.len() - 1)
            .expect_err("an unrestorable output must be rejected");

        assert!(matches!(error, TransferError::BackupSize));
    }

    #[test]
    fn rejects_an_unrenderable_backup_envelope_timestamp() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "format": "crane-backup",
            "backupVersion": 1,
            "createdAt": 8_640_000_000_000_001_u64,
            "appVersion": "0.1.0",
            "store": Store::seed(10)
        }))
        .expect("fixture should serialize");

        let error = inspect_backup(&bytes, "token".into())
            .expect_err("an unrenderable backup date must fail before confirmation");

        assert!(matches!(error, TransferError::Validation(_)));
    }

    #[test]
    fn writes_safety_backup_before_replacement() {
        let phases = RefCell::new(Vec::new());
        let current = Store::seed(10);
        let mut replacement = Store::seed(20);
        replacement.notes.clear();

        let restored = restore_transaction(
            &current,
            replacement,
            |_| {
                phases.borrow_mut().push("safety");
                Ok(())
            },
            |_| {
                phases.borrow_mut().push("replacement");
                Ok(())
            },
        )
        .expect("both writes should succeed");

        assert_eq!(*phases.borrow(), ["safety", "replacement"]);
        assert!(restored.notes.is_empty());
    }

    #[test]
    fn safety_failure_skips_replacement() {
        let replacement_called = Cell::new(false);
        let error = restore_transaction(
            &Store::seed(10),
            Store::seed(20),
            |_| Err(TransferError::Io("safety failed".into())),
            |_| {
                replacement_called.set(true);
                Ok(())
            },
        )
        .expect_err("a safety-backup failure must abort restore");

        assert!(matches!(error, TransferError::Io(_)));
        assert!(!replacement_called.get());
    }

    #[test]
    fn replacement_failure_returns_error_without_new_in_memory_store() {
        let current = Store::seed(10);
        let result = restore_transaction(
            &current,
            Store::seed(20),
            |_| Ok(()),
            |_| Err(TransferError::Io("replacement failed".into())),
        );

        assert!(matches!(result, Err(TransferError::Io(_))));
        assert_eq!(current.notes[0].updated_at, 10u64.saturating_sub(3_600_000));
    }

    fn backup_json(format: &str, backup_version: u32, schema_version: u32) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": format,
            "backupVersion": backup_version,
            "createdAt": 1,
            "appVersion": "0.1.0",
            "store": { "schemaVersion": schema_version, "notes": [] }
        }))
        .expect("test JSON should serialize")
    }

    fn unchecked_backup_json(store: &Store) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "format": "crane-backup",
            "backupVersion": 1,
            "createdAt": 1,
            "appVersion": "0.1.0",
            "store": store
        }))
        .expect("test JSON should serialize")
    }
}
