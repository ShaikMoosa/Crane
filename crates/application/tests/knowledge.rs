mod support;

use crane_application::{Command, CommandData, ErrorCode, Library, NoteState};
use crane_domain::{Note, Store, SCHEMA_VERSION};
use support::{FakeEnvironment, FakeRepository};

fn note(id: &str, title: &str, body: &str, updated_at: u64) -> Note {
    Note {
        id: id.into(),
        title: title.into(),
        body: body.into(),
        tags: crane_domain::extract_tags(body),
        created_at: 1,
        updated_at,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

fn open(notes: Vec<Note>) -> (Library, FakeRepository) {
    let store = Store {
        schema_version: SCHEMA_VERSION,
        notes,
        extensions: Default::default(),
    };
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
fn index_is_built_on_open_and_follows_commits() {
    let (mut library, _repo) = open(vec![
        note("a", "Target", "Body", 1),
        note("b", "Source", "[[Target]]", 2),
    ]);
    assert_eq!(library.knowledge_index().note_count(), 2);
    assert_eq!(library.knowledge_index().backlinks("a").len(), 1);

    let result = library
        .execute(Command::CreateNoteWithTitle {
            title: "  Linked Note  ".into(),
        })
        .unwrap();
    let CommandData::Created(created) = result.data else {
        panic!("expected Created");
    };
    assert_eq!(created.title, "Linked Note");
    assert!(library.knowledge_index().facts(&created.id).is_some());

    library
        .execute(Command::SaveNote {
            id: "b".into(),
            title: "Source".into(),
            body: "[[Linked Note]]".into(),
            draft_revision: 1,
        })
        .unwrap();
    assert_eq!(library.knowledge_index().backlinks(&created.id).len(), 1);
    assert!(library.knowledge_index().backlinks("a").is_empty());

    library
        .execute(Command::SetNoteState {
            id: "b".into(),
            state: NoteState::Trashed,
            value: true,
        })
        .unwrap();
    assert!(library.knowledge_index().visibility("b").unwrap().trashed);
    assert_eq!(library.knowledge_index().facts("b").unwrap().links.len(), 1);
}

#[test]
fn failed_commit_leaves_index_unchanged_and_empty_title_is_rejected() {
    let (mut library, repo) = open(vec![note("a", "Target", "Body", 1)]);
    repo.fail_next_commit();
    let before = library.knowledge_index().note_count();
    assert!(library
        .execute(Command::CreateNoteWithTitle {
            title: "Nope".into()
        })
        .is_err());
    assert_eq!(library.knowledge_index().note_count(), before);

    let error = library
        .execute(Command::CreateNoteWithTitle {
            title: "   ".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Validation);
}
