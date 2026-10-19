mod support;

use crane_application::{Command, CommandData, ErrorCode, Library, NoteState};
use crane_domain::{Store, MAX_BODY_BYTES};
use support::{FakeEnvironment, FakeRepository};

fn extended_store() -> Store {
    serde_json::from_str(include_str!("../../../fixtures/compat/v1/extended.json")).unwrap()
}

fn open(store: Store, now: u64) -> (Library, FakeRepository) {
    let repository = FakeRepository::new(store);
    let handle = repository.clone();
    let library = Library::open(
        Box::new(repository),
        Box::new(FakeEnvironment::new(now, 10)),
    )
    .unwrap();
    (library, handle)
}

fn snapshot_json(library: &Library) -> serde_json::Value {
    serde_json::to_value(library.snapshot().store).unwrap()
}

#[test]
fn failed_create_save_and_state_changes_preserve_committed_store() {
    let commands = [
        Command::CreateNote,
        Command::SaveNote {
            id: "legacy-active".into(),
            title: "Changed".into(),
            body: "Changed #tag".into(),
            draft_revision: 7,
        },
        Command::SetNoteState {
            id: "legacy-active".into(),
            state: NoteState::Pinned,
            value: true,
        },
    ];

    for command in commands {
        let (mut library, repository) = open(extended_store(), 500);
        let memory_before = snapshot_json(&library);
        let disk_before = repository.disk_bytes();
        let revision_before = library.snapshot().revision;
        repository.fail_next_commit();

        let error = library
            .execute(command)
            .expect_err("the injected commit failure must surface");

        assert_eq!(error.code, ErrorCode::Io);
        assert_eq!(snapshot_json(&library), memory_before);
        assert_eq!(repository.disk_bytes(), disk_before);
        assert_eq!(library.snapshot().revision, revision_before);
        assert_eq!(repository.commit_count(), 0);
    }
}

#[test]
fn create_commits_once_and_publishes_after_success() {
    let (mut library, repository) = open(extended_store(), 700);

    let result = library.execute(Command::CreateNote).unwrap();

    assert_eq!(result.revision, 1);
    assert_eq!(repository.commit_count(), 1);
    let CommandData::Created(note) = result.data else {
        panic!("create must return the created note");
    };
    assert_eq!(note.id, "fake-note-1");
    assert_eq!(note.created_at, 700);
    assert_eq!(note.updated_at, 700);
    assert_eq!(library.snapshot().store.notes.last().unwrap().id, note.id);
}

#[test]
fn save_updates_owned_fields_and_preserves_identity_extensions_and_flags() {
    let (mut library, repository) = open(extended_store(), 900);
    let before = library.snapshot().store.notes[0].clone();

    let result = library
        .execute(Command::SaveNote {
            id: before.id.clone(),
            title: "Revised".into(),
            body: "Text #Work/Notes #work/notes".into(),
            draft_revision: 7,
        })
        .unwrap();

    assert_eq!(result.revision, 1);
    assert_eq!(repository.commit_count(), 1);
    let CommandData::Saved {
        note,
        draft_revision,
    } = result.data
    else {
        panic!("save must return the saved note and draft revision");
    };
    assert_eq!(draft_revision, 7);
    assert_eq!(note.id, before.id);
    assert_eq!(note.created_at, before.created_at);
    assert_eq!(note.updated_at, 900);
    assert_eq!(note.tags, ["work/notes"]);
    assert_eq!(note.extensions, before.extensions);
    assert_eq!(note.pinned, before.pinned);
    assert_eq!(
        library.snapshot().store.extensions["futureStore"]["enabled"],
        true
    );
}

#[test]
fn state_change_commits_once_and_updates_timestamp() {
    let (mut library, repository) = open(extended_store(), 1_100);

    let result = library
        .execute(Command::SetNoteState {
            id: "legacy-active".into(),
            state: NoteState::Archived,
            value: true,
        })
        .unwrap();

    let CommandData::StateChanged(note) = result.data else {
        panic!("state mutation must return the changed note");
    };
    assert!(note.archived);
    assert_eq!(note.updated_at, 1_100);
    assert_eq!(repository.commit_count(), 1);
    assert_eq!(result.revision, 1);
}

#[test]
fn invalid_or_missing_input_never_commits() {
    let (mut library, repository) = open(extended_store(), 1_200);

    let too_large = library
        .execute(Command::SaveNote {
            id: "legacy-active".into(),
            title: "Title".into(),
            body: "x".repeat(MAX_BODY_BYTES + 1),
            draft_revision: 8,
        })
        .unwrap_err();
    assert_eq!(too_large.code, ErrorCode::Validation);

    let missing = library
        .execute(Command::SetNoteState {
            id: "missing".into(),
            state: NoteState::Trashed,
            value: true,
        })
        .unwrap_err();
    assert_eq!(missing.code, ErrorCode::NotFound);
    assert_eq!(repository.commit_count(), 0);
    assert_eq!(library.snapshot().revision, 0);
}
