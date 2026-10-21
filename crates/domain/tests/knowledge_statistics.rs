use crane_domain::knowledge::{note_statistics, parse_draft_facts, parse_note_facts};
use crane_domain::Note;

fn note(body: &str) -> Note {
    Note {
        id: "n1".into(),
        title: "Field Notes".into(),
        body: body.into(),
        tags: vec!["stored/tag".into()],
        created_at: 1,
        updated_at: 2,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

#[test]
fn counts_statistics_exactly() {
    let stats = note_statistics("# T\n\nOne two three.\n\n- [ ] a\n- [x] b\n");
    assert_eq!(stats.headings, 1);
    assert_eq!(stats.tasks, 2);
    assert_eq!(stats.completed_tasks, 1);
    assert_eq!(stats.words, 12);
    assert_eq!(stats.paragraphs, 1);
    assert_eq!(stats.reading_minutes, 1);
    let empty = note_statistics("");
    assert_eq!(empty.words, 0);
    assert_eq!(empty.reading_minutes, 0);
    assert_eq!(note_statistics(&"word ".repeat(201)).reading_minutes, 2);
}

#[test]
fn code_blocks_do_not_count_tasks_or_links() {
    let body = "```\n- [ ] hidden\n[[Hidden]]\n```\n\n- [ ] shown\n";
    let facts = parse_note_facts(&note(body));
    assert_eq!(facts.statistics.tasks, 1);
    assert!(facts.links.is_empty());
}

#[test]
fn facts_keep_stored_tags_and_draft_rederives() {
    let committed = parse_note_facts(&note("Body #fresh/tag"));
    assert_eq!(committed.tags, ["stored/tag"]);
    assert_eq!(committed.normalized_title, "field notes");
    assert_eq!(committed.search_body, "body #fresh/tag");
    let draft = parse_draft_facts("n1", "Field Notes", "Body #fresh/tag");
    assert_eq!(draft.tags, ["fresh/tag"]);
    let body = "Body #fresh/tag".to_string();
    let before = body.clone();
    let _ = parse_draft_facts("n1", "T", &body);
    assert_eq!(body, before);
}
