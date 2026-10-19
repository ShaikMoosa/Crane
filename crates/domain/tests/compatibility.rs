use crane_domain::{
    extract_tags, validate_note, validate_store, Note, Store, ValidationCode, MAX_BODY_BYTES,
    MAX_RENDERABLE_TIMESTAMP, MAX_TITLE_BYTES, SCHEMA_VERSION,
};

fn note(id: &str) -> Note {
    Note {
        id: id.into(),
        title: "Title".into(),
        body: "Body".into(),
        tags: vec![],
        created_at: 0,
        updated_at: 0,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

#[test]
fn supported_v1_fixtures_round_trip_semantically() {
    for source in [
        include_str!("../../../fixtures/compat/v1/all-states.json"),
        include_str!("../../../fixtures/compat/v1/empty.json"),
        include_str!("../../../fixtures/compat/v1/extended.json"),
    ] {
        let original: serde_json::Value = serde_json::from_str(source).unwrap();
        let store: Store = serde_json::from_value(original.clone()).unwrap();

        validate_store(&store).expect("the supported fixture must validate");
        assert_eq!(serde_json::to_value(&store).unwrap(), original);
        assert_eq!(store.schema_version, SCHEMA_VERSION);
    }
}

#[test]
fn unknown_store_and_note_fields_survive_round_trip() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/compat/v1/extended.json")).unwrap();
    let store: Store = serde_json::from_value(original.clone()).unwrap();

    assert_eq!(serde_json::to_value(&store).unwrap(), original);
    assert_eq!(store.extensions["futureStore"]["source"], "synthetic");
    assert_eq!(store.notes[0].extensions["futureNote"]["rank"], 7);
}

#[test]
fn future_schema_is_rejected_without_reinterpretation() {
    let store: Store = serde_json::from_str(include_str!(
        "../../../fixtures/compat/v1/future-schema.json"
    ))
    .unwrap();

    let error = validate_store(&store).expect_err("future schemas must fail visibly");
    assert_eq!(error.code, ValidationCode::UnsupportedSchema);
}

#[test]
fn title_and_body_limits_are_exact_utf8_byte_limits() {
    assert_eq!(MAX_TITLE_BYTES, 500);
    assert_eq!(MAX_BODY_BYTES, 2 * 1024 * 1024);

    let mut candidate = note("limits");
    candidate.title = "é".repeat(MAX_TITLE_BYTES / 2);
    candidate.body = "x".repeat(MAX_BODY_BYTES);
    validate_note(&candidate).expect("values exactly at byte limits must pass");

    candidate.title.push('x');
    assert_eq!(
        validate_note(&candidate).unwrap_err().code,
        ValidationCode::TitleTooLong
    );

    candidate.title = "ok".into();
    candidate.body.push('x');
    assert_eq!(
        validate_note(&candidate).unwrap_err().code,
        ValidationCode::BodyTooLarge
    );
}

#[test]
fn blank_and_duplicate_ids_are_rejected() {
    let mut blank = note("   ");
    assert_eq!(
        validate_note(&blank).unwrap_err().code,
        ValidationCode::MissingId
    );

    blank.id = "same".into();
    let store = Store {
        schema_version: SCHEMA_VERSION,
        notes: vec![blank.clone(), blank],
        extensions: Default::default(),
    };
    assert_eq!(
        validate_store(&store).unwrap_err().code,
        ValidationCode::DuplicateId
    );
}

#[test]
fn timestamps_respect_the_renderable_boundary() {
    let mut candidate = note("timestamp");
    candidate.created_at = MAX_RENDERABLE_TIMESTAMP;
    candidate.updated_at = MAX_RENDERABLE_TIMESTAMP;
    validate_note(&candidate).expect("the maximum renderable timestamp must pass");

    candidate.updated_at += 1;
    assert_eq!(
        validate_note(&candidate).unwrap_err().code,
        ValidationCode::TimestampOutOfRange
    );
}

#[test]
fn seed_is_valid_and_contains_no_extension_data() {
    let store = Store::seed(100);
    validate_store(&store).expect("seed data must remain valid");
    assert!(store.extensions.is_empty());
    assert!(store.notes.iter().all(|note| note.extensions.is_empty()));
}

#[test]
fn extracts_unique_nested_tags() {
    assert_eq!(
        extract_tags("#Design and #work/Notes, then #design"),
        vec!["design", "work/notes"]
    );
}

#[test]
fn extracts_hyphenated_nested_and_unicode_tags_with_legacy_grammar() {
    assert_eq!(
        extract_tags("#Work-Flow #work/Notes, #naïve。 #日本語。 #work//bad (#ignored/tag)"),
        vec!["work-flow", "work/notes", "naïve", "日本語"]
    );
}
