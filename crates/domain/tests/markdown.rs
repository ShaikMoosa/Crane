use crane_domain::{
    build_export_manifest, excerpt_from, parse_markdown, prepare_import, serialize_markdown,
    NamedBytes, Store, MAX_BATCH_BYTES,
};

#[test]
fn imports_heading_as_title_and_preserves_body() {
    let note = parse_markdown(
        "idea.md",
        b"# A title\n\nBody #work/notes",
        10,
        "one".into(),
    )
    .unwrap();
    assert_eq!(note.title, "A title");
    assert_eq!(note.body, "Body #work/notes");
    assert_eq!(note.tags, ["work/notes"]);
    assert_eq!(note.created_at, 10);
    assert_eq!(serialize_markdown(&note), "# A title\n\nBody #work/notes");
}

#[test]
fn import_uses_filename_when_h1_is_absent() {
    let note = parse_markdown("Field notes.md", b"Body", 10, "one".into()).unwrap();
    assert_eq!(note.title, "Field notes");
    assert_eq!(note.body, "Body");
}

#[test]
fn invalid_file_aborts_the_entire_batch() {
    let files = vec![
        NamedBytes {
            name: "one.md".into(),
            bytes: b"One".to_vec(),
        },
        NamedBytes {
            name: "two.txt".into(),
            bytes: b"Two".to_vec(),
        },
    ];
    assert!(prepare_import(files, 3, ["one".into(), "two".into()].into_iter()).is_err());
}

#[test]
fn rejects_non_utf8_and_oversized_batches() {
    assert!(parse_markdown("bad.md", &[0xff], 1, "one".into()).is_err());
    assert!(prepare_import(
        vec![NamedBytes {
            name: "huge.md".into(),
            bytes: vec![b'x'; MAX_BATCH_BYTES + 1]
        }],
        1,
        ["one".into()].into_iter(),
    )
    .is_err());
}

#[test]
fn export_manifest_separates_active_and_archive_and_excludes_trash() {
    let mut store = Store::seed(100);
    store.notes[0].title = "Active".into();
    store.notes[1].title = "Archived".into();
    store.notes[1].archived = true;
    store.notes[2].trashed = true;

    let manifest = build_export_manifest(&store).unwrap();
    let paths: Vec<_> = manifest
        .entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();
    assert_eq!(paths, ["Notes/Active.md", "Archive/Archived.md"]);
}

#[test]
fn export_names_are_windows_safe_and_collision_resistant() {
    let mut store = Store::seed(100);
    let titles = ["CON", "Plan", "plan", "Σ", "ς", "COM¹"];
    store.notes = titles
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let mut note = Store::seed(index as u64).notes.remove(0);
            note.id = format!("note-{index}");
            note.title = title.into();
            note
        })
        .collect();

    let manifest = build_export_manifest(&store).unwrap();
    let paths: Vec<_> = manifest
        .entries
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "Notes/Untitled note.md",
            "Notes/Plan.md",
            "Notes/plan 2.md",
            "Notes/Σ.md",
            "Notes/ς 2.md",
            "Notes/Untitled note 2.md",
        ]
    );
}

#[test]
fn excerpt_removes_markup_code_images_and_tags() {
    assert_eq!(
        excerpt_from("## Hello\n\n**Strong** words with #tag", 116),
        "Hello Strong words with"
    );
    assert_eq!(excerpt_from("#only-tag", 116), "No additional text");
    assert_eq!(
        excerpt_from(
            "```rust\nhidden\n```\n![alt](https://example.com/a.png) Visible",
            116
        ),
        "Visible"
    );
}
