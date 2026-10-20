use crane_domain::{query_note_ids, Collection, Note, NoteFilter, Store, SCHEMA_VERSION};

fn note(id: &str, title: &str, body: &str, updated_at: u64) -> Note {
    Note {
        id: id.into(),
        title: title.into(),
        body: body.into(),
        tags: crane_domain::extract_tags(body),
        created_at: 0,
        updated_at,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

fn store() -> Store {
    let a = note(
        "a",
        "Quiet systems",
        "A note about #Design and #work/notes.",
        30,
    );
    let mut b = note("b", "Pinned idea", "Keep this close. #Ideas", 10);
    b.pinned = true;
    let mut c = note("c", "Old draft", "Archived material #work/notes", 50);
    c.archived = true;
    let mut d = note("d", "Discarded systems", "Recover me #work/notes", 60);
    d.trashed = true;
    Store {
        schema_version: SCHEMA_VERSION,
        notes: vec![a, b, c, d],
        extensions: Default::default(),
    }
}

fn filter(collection: Collection, tag: Option<&str>, query: &str) -> NoteFilter {
    NoteFilter {
        collection,
        tag: tag.map(str::to_string),
        query: query.into(),
    }
}

#[test]
fn active_notes_sort_pinned_first_then_by_recency() {
    assert_eq!(
        query_note_ids(&store(), &filter(Collection::Notes, None, "")),
        ["b", "a"]
    );
}

#[test]
fn collections_keep_active_archive_and_trash_separate() {
    assert_eq!(
        query_note_ids(&store(), &filter(Collection::Pinned, None, "")),
        ["b"]
    );
    assert_eq!(
        query_note_ids(&store(), &filter(Collection::Archive, None, "")),
        ["c"]
    );
    assert_eq!(
        query_note_ids(&store(), &filter(Collection::Trash, None, "")),
        ["d"]
    );
}

#[test]
fn search_combines_title_body_and_tags() {
    assert_eq!(
        query_note_ids(
            &store(),
            &filter(Collection::Notes, Some("WORK/NOTES"), "systems")
        ),
        ["a"]
    );
    assert_eq!(
        query_note_ids(&store(), &filter(Collection::Notes, None, "ideas")),
        ["b"]
    );
}

#[test]
fn trash_applies_tag_and_query_filters_instead_of_returning_early() {
    assert_eq!(
        query_note_ids(
            &store(),
            &filter(Collection::Trash, Some("work/notes"), "systems")
        ),
        ["d"]
    );
    assert!(query_note_ids(
        &store(),
        &filter(Collection::Trash, Some("work/notes"), "absent")
    )
    .is_empty());
}

#[test]
fn equal_sort_keys_keep_original_store_order() {
    let mut source = store();
    source.notes[0].pinned = true;
    source.notes[1].updated_at = source.notes[0].updated_at;
    assert_eq!(
        query_note_ids(&source, &filter(Collection::Notes, None, "")),
        ["a", "b"]
    );
}
