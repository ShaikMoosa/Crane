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

use crane_application::knowledge::{CollectionView, LinkState, Query, QueryData, ResolutionView};

fn query_ids(library: &mut Library, collection: CollectionView) -> Vec<String> {
    let response = library.query(Query::NotesForCollection {
        collection,
        tag_path: None,
        query: String::new(),
    });
    let QueryData::NoteIds(ids) = response.data else {
        panic!("expected NoteIds");
    };
    ids
}

#[test]
fn quick_open_ranks_and_scopes() {
    let (mut library, _repo) = open(vec![
        note("exact", "Alpha", "body", 10),
        note("prefix", "Alphabet", "body", 90),
        note("taggy", "Other", "tagged #alpha topic", 80),
        note("arch", "Alpha archive", "body", 70),
    ]);
    library
        .execute(Command::SetNoteState {
            id: "arch".into(),
            state: NoteState::Archived,
            value: true,
        })
        .unwrap();
    let response = library.query(Query::QuickOpen {
        query: "alpha".into(),
        include_archived: false,
    });
    let QueryData::QuickOpen(hits) = response.data else {
        panic!("expected hits");
    };
    let ids: Vec<_> = hits.iter().map(|h| h.note_id.as_str()).collect();
    assert_eq!(ids, ["exact", "prefix", "taggy"]);
    let response = library.query(Query::QuickOpen {
        query: "alpha".into(),
        include_archived: true,
    });
    let QueryData::QuickOpen(hits) = response.data else {
        panic!("expected hits");
    };
    assert_eq!(hits.len(), 4);
}

#[test]
fn resolution_and_knowledge_views_are_explicit() {
    let (mut library, _repo) = open(vec![
        note("a", "Field Notes", "# Intro\n\nBody", 1),
        note("dup", "field notes", "Copy", 2),
        note(
            "src",
            "Source",
            "[[Field Notes]] [[Field Notes#Intro]] [[Ghost]]",
            3,
        ),
    ]);
    let source = library.knowledge_index().facts("src").unwrap().clone();
    let response = library.query(Query::ResolveWikiLink {
        link: source.links[0].clone(),
    });
    assert!(matches!(
        response.data,
        QueryData::Resolution(ResolutionView::Ambiguous { .. })
    ));
    let response = library.query(Query::KnowledgeFor {
        note_id: "src".into(),
    });
    let QueryData::Knowledge(view) = response.data else {
        panic!("expected knowledge");
    };
    assert!(matches!(view.outgoing[0].state, LinkState::Ambiguous));
    assert!(matches!(view.outgoing[2].state, LinkState::Unresolved));
    assert!(view.backlinks.is_empty());
}

#[test]
fn derived_collections_and_tag_tree() {
    let (mut library, _repo) = open(vec![
        note("today", "Today note", "fresh #project/crane/ui", 500),
        note("todo", "Todo note", "- [ ] open task", 400),
        note("plain", "Plain", "no tags here", 300),
        note("old", "Old", "stale #project/web", 10),
    ]);
    assert_eq!(query_ids(&mut library, CollectionView::Today).len(), 4);
    assert_eq!(query_ids(&mut library, CollectionView::Todos), ["todo"]);
    assert_eq!(
        query_ids(&mut library, CollectionView::Untagged),
        ["todo", "plain"]
    );
    let response = library.query(Query::NotesForCollection {
        collection: CollectionView::Notes,
        tag_path: Some("project".into()),
        query: String::new(),
    });
    let QueryData::NoteIds(ids) = response.data else {
        panic!("expected ids");
    };
    assert_eq!(ids, ["today", "old"]);
    let response = library.query(Query::TagTree {
        include_archived: false,
    });
    let QueryData::TagTree(tree) = response.data else {
        panic!("expected tree");
    };
    assert_eq!(tree[0].path, "project");
    assert_eq!(tree[0].note_count, 2);
}

#[test]
fn completion_switches_to_headings() {
    let (mut library, _repo) = open(vec![
        note("a", "Field Notes", "# Intro\n\n## On the desk\n", 1),
        note("b", "Field Trip", "Body", 2),
    ]);
    let response = library.query(Query::CompleteWikiLink {
        fragment: "Fie".into(),
    });
    let QueryData::Completion(candidates) = response.data else {
        panic!("expected completion");
    };
    assert_eq!(candidates.len(), 2);
    let response = library.query(Query::CompleteWikiLink {
        fragment: "Field Notes#On".into(),
    });
    let QueryData::Completion(candidates) = response.data else {
        panic!("expected completion");
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].insert_text(), "On the desk");
    let response = library.query(Query::CompleteWikiLink {
        fragment: "Field Notes|alias".into(),
    });
    let QueryData::Completion(candidates) = response.data else {
        panic!("expected completion");
    };
    assert!(candidates.is_empty());
}
