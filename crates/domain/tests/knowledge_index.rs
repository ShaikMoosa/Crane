use crane_domain::knowledge::{HeadingResolution, KnowledgeIndex, Resolution};
use crane_domain::{Note, Store, SCHEMA_VERSION};

fn note(id: &str, title: &str, body: &str) -> Note {
    Note {
        id: id.into(),
        title: title.into(),
        body: body.into(),
        tags: crane_domain::extract_tags(body),
        created_at: 1,
        updated_at: 1,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    }
}

fn store(notes: Vec<Note>) -> Store {
    Store {
        schema_version: SCHEMA_VERSION,
        notes,
        extensions: Default::default(),
    }
}

#[test]
fn resolves_unique_missing_and_ambiguous() {
    let index = KnowledgeIndex::build(&store(vec![
        note("a", "Field Notes", "# Intro\n\nBody"),
        note("b", "field  notes", "Duplicate"),
        note(
            "c",
            "Source",
            "See [[Field Notes#Intro]] and [[Field Notes#Nope]] and [[Ghost]]",
        ),
    ]));
    let source = index.facts("c").unwrap();
    assert!(matches!(
        index.resolve(&source.links[0]),
        Resolution::Ambiguous { .. }
    ));
    let unique = KnowledgeIndex::build(&store(vec![
        note("a", "Field Notes", "# Intro\n\nBody"),
        note(
            "c",
            "Source",
            "See [[Field Notes#Intro]] and [[Field Notes#Nope]] and [[Ghost]]",
        ),
    ]));
    let source = unique.facts("c").unwrap();
    assert!(matches!(
        unique.resolve(&source.links[0]),
        Resolution::Resolved {
            heading: HeadingResolution::Found { .. },
            ..
        }
    ));
    assert!(matches!(
        unique.resolve(&source.links[1]),
        Resolution::Resolved {
            heading: HeadingResolution::Missing,
            ..
        }
    ));
    assert!(matches!(
        unique.resolve(&source.links[2]),
        Resolution::Unresolved { .. }
    ));
}

#[test]
fn backlinks_track_renames_and_visibility() {
    let mut index = KnowledgeIndex::build(&store(vec![
        note("a", "Target", "Body"),
        note("c", "Source", "Link to [[Target]] here please"),
    ]));
    assert_eq!(index.backlinks("a").len(), 1);
    assert!(index.backlinks("a")[0].context_excerpt.contains("Link to"));
    // Rename the target: the inbound link becomes unresolved, no stale backlink.
    let mut renamed = note("a", "Renamed", "Body");
    renamed.updated_at = 9;
    index.replace_note(&renamed);
    assert!(index.backlinks("a").is_empty());
    // Visibility flips without reparsing.
    let mut trashed = note("c", "Source", "Link to [[Target]] here please");
    trashed.trashed = true;
    index.update_visibility(&trashed);
    assert_eq!(index.visibility("c").unwrap().trashed, true);
    assert_eq!(index.facts("c").unwrap().links.len(), 1);
}

#[test]
fn empty_titles_are_never_link_targets() {
    let index = KnowledgeIndex::build(&store(vec![
        note("a", "  ", "Body"),
        note("c", "Source", "[[x]]"),
    ]));
    assert!(index.facts("a").is_some());
    let source = index.facts("c").unwrap();
    assert!(matches!(
        index.resolve(&source.links[0]),
        Resolution::Unresolved { .. }
    ));
}
