mod support;

use crane_application::knowledge::{Query, QueryData, QUICK_OPEN_LIMIT};
use crane_application::{Command, Library};
use crane_domain::{
    inspect_backup, make_backup, serialize_backup, Note, Store, BACKUP_VERSION, SCHEMA_VERSION,
};
use std::time::Instant;
use support::{FakeEnvironment, FakeRepository};

const NOTE_COUNT: usize = 5_000;

fn note(id: String, title: String, body: String, updated_at: u64) -> Note {
    Note {
        id,
        title,
        tags: crane_domain::extract_tags(&body),
        body,
        created_at: 1,
        updated_at,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

/// Deterministic mixed store: headings, `[[links]]`, nested tags, and tasks.
fn large_store() -> Store {
    let mut notes = Vec::with_capacity(NOTE_COUNT);
    for i in 0..NOTE_COUNT {
        let id = format!("n{i:04}");
        let title = format!("Note {i:04}");
        let mut body = format!("# Heading {i}\n\nBody of note {i}.\n");
        if i % 2 == 0 {
            let next = (i + 1) % NOTE_COUNT;
            body.push_str(&format!("See also [[Note {next:04}]].\n"));
        }
        body.push_str(&format!("#topic/group{:02}\n", i % 50));
        if i % 3 == 0 {
            body.push_str(&format!("- [ ] task {i}\n"));
        }
        if i == 0 {
            body.push_str(&"x".repeat(63_000));
        }
        notes.push(note(id, title, body, i as u64));
    }
    Store {
        schema_version: SCHEMA_VERSION,
        notes,
        extensions: Default::default(),
    }
}

fn open(store: Store) -> (Library, FakeRepository) {
    let repository = FakeRepository::new(store);
    let handle = repository.clone();
    let library = Library::open(
        Box::new(repository),
        Box::new(FakeEnvironment::new(10_000, 10)),
    )
    .unwrap();
    (library, handle)
}

#[test]
fn large_library_index_build_query_and_save_counts() {
    let store = large_store();
    assert_eq!(store.notes.len(), NOTE_COUNT);
    assert!(store.notes[0].body.len() > 60_000);

    let started = Instant::now();
    let (mut library, repository) = open(store);
    let index = library.knowledge_index();
    assert_eq!(index.note_count(), NOTE_COUNT);
    assert!(index
        .facts("n0000")
        .is_some_and(|facts| !facts.links.is_empty()));
    eprintln!(
        "perf: open+index {NOTE_COUNT} notes in {:?}",
        started.elapsed()
    );

    let started = Instant::now();
    for i in 0..200 {
        let response = library.query(Query::QuickOpen {
            query: format!("note {i:04}"),
            include_archived: false,
        });
        let QueryData::QuickOpen(hits) = response.data else {
            panic!("expected quick-open hits");
        };
        assert!(
            hits.len() <= QUICK_OPEN_LIMIT,
            "quick-open hits must stay bounded"
        );
    }
    let response = library.query(Query::QuickOpen {
        query: "note".to_owned(),
        include_archived: false,
    });
    let QueryData::QuickOpen(hits) = response.data else {
        panic!("expected quick-open hits");
    };
    assert_eq!(hits.len(), QUICK_OPEN_LIMIT);
    eprintln!("perf: 200 quick-open queries in {:?}", started.elapsed());

    let before_other = library
        .knowledge_index()
        .facts("n0002")
        .expect("other facts")
        .links
        .len();
    let started = Instant::now();
    library
        .execute(Command::SaveNote {
            id: "n0000".to_owned(),
            title: "Note 0000".to_owned(),
            body: "plain replacement body".to_owned(),
            draft_revision: 1,
        })
        .unwrap();
    eprintln!("perf: save+reindex one note in {:?}", started.elapsed());
    assert_eq!(repository.commit_count(), 1);

    let index = library.knowledge_index();
    assert_eq!(index.note_count(), NOTE_COUNT);
    assert!(index
        .facts("n0000")
        .is_some_and(|facts| facts.links.is_empty()));
    assert!(!index
        .backlinks("n0001")
        .iter()
        .any(|inbound| inbound.source_note_id == "n0000"));
    assert_eq!(
        index.facts("n0002").expect("other facts").links.len(),
        before_other
    );
}

fn fixture_value(name: &str) -> serde_json::Value {
    let source = match name {
        "all-states.json" => include_str!("../../../fixtures/compat/v1/all-states.json"),
        "empty.json" => include_str!("../../../fixtures/compat/v1/empty.json"),
        "extended.json" => include_str!("../../../fixtures/compat/v1/extended.json"),
        "backup.json" => include_str!("../../../fixtures/compat/v1/backup.json"),
        "future-schema.json" => include_str!("../../../fixtures/compat/v1/future-schema.json"),
        _ => panic!("unknown fixture {name}"),
    };
    serde_json::from_str(source).unwrap()
}

#[test]
fn v1_fixtures_open_backup_and_restore_unchanged() {
    for name in ["all-states.json", "empty.json", "extended.json"] {
        let original = fixture_value(name);
        let store: Store = serde_json::from_value(original.clone()).unwrap();
        let (library, _) = open(store);
        let snapshot = library.snapshot();
        assert_eq!(
            serde_json::to_value(&snapshot.store).unwrap(),
            original,
            "{name} must survive a Library open byte-identically"
        );

        let envelope = make_backup(&snapshot.store, 500, "0.3.0").unwrap();
        assert_eq!(envelope.app_version, "0.3.0");
        assert_eq!(envelope.backup_version, BACKUP_VERSION);
        assert_eq!(BACKUP_VERSION, 1);
        assert_eq!(snapshot.store.schema_version, SCHEMA_VERSION);
        assert_eq!(SCHEMA_VERSION, 1);
        let bytes = serialize_backup(&envelope).unwrap();
        let (restored, _) = inspect_backup(&bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&restored.store).unwrap(),
            original,
            "{name} backup must restore byte-identically"
        );
    }

    let legacy_backup = fixture_value("backup.json");
    let bytes = serde_json::to_vec(&legacy_backup).unwrap();
    let (envelope, _) = inspect_backup(&bytes).unwrap();
    assert_eq!(envelope.backup_version, BACKUP_VERSION);

    let future: Store = serde_json::from_value(fixture_value("future-schema.json")).unwrap();
    let repository = FakeRepository::new(future);
    assert!(
        Library::open(
            Box::new(repository),
            Box::new(FakeEnvironment::new(10_000, 10)),
        )
        .is_err(),
        "future schemas must fail to open"
    );
}
