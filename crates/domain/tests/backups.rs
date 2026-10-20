use crane_domain::{
    inspect_backup, make_backup, serialize_backup, BackupEnvelope, MAX_BACKUP_BYTES,
    MAX_RENDERABLE_TIMESTAMP,
};

#[test]
fn version_one_backup_round_trip_preserves_all_unknown_fields() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/compat/v1/backup.json")).unwrap();
    let envelope: BackupEnvelope = serde_json::from_value(original.clone()).unwrap();
    let bytes = serialize_backup(&envelope).unwrap();
    let (parsed, inspection) = inspect_backup(&bytes).unwrap();

    assert_eq!(serde_json::to_value(parsed).unwrap(), original);
    assert_eq!(inspection.total, 1);
    assert_eq!(inspection.active, 1);
    assert_eq!(inspection.archived, 0);
    assert_eq!(inspection.trashed, 0);
}

#[test]
fn backup_builder_counts_every_note_state() {
    let mut store = crane_domain::Store::seed(100);
    store.notes[1].archived = true;
    store.notes[2].trashed = true;
    let envelope = make_backup(&store, 500, "0.2.0").unwrap();
    let (_, inspection) = inspect_backup(&serialize_backup(&envelope).unwrap()).unwrap();
    assert_eq!(
        (
            inspection.total,
            inspection.active,
            inspection.archived,
            inspection.trashed
        ),
        (3, 1, 1, 1)
    );
}

#[test]
fn malformed_future_and_oversized_backups_are_rejected() {
    assert!(inspect_backup(b"not JSON").is_err());
    let future = br#"{"format":"crane-backup","backupVersion":2,"createdAt":1,"appVersion":"x","store":{"schemaVersion":1,"notes":[]}}"#;
    assert!(inspect_backup(future).is_err());
    assert!(inspect_backup(&vec![b' '; MAX_BACKUP_BYTES + 1]).is_err());
}

#[test]
fn invalid_backup_timestamp_is_rejected() {
    let store = crane_domain::Store::seed(1);
    assert!(make_backup(&store, MAX_RENDERABLE_TIMESTAMP + 1, "0.2.0").is_err());
}
