use crane_domain::knowledge::{build_tag_tree, tag_in_subtree};

#[test]
fn builds_hierarchy_with_distinct_counts() {
    let tree = build_tag_tree(&[
        vec!["project/crane/ui".into(), "project/web".into()],
        vec!["project/crane".into()],
        vec!["reading".into()],
    ]);
    assert_eq!(tree.len(), 2);
    let project = &tree[0];
    assert_eq!(project.path, "project");
    assert_eq!(project.note_count, 2);
    let crane = &project.children[0];
    assert_eq!(crane.path, "project/crane");
    assert_eq!(crane.note_count, 2);
    assert_eq!(crane.children[0].path, "project/crane/ui");
    assert_eq!(crane.children[0].note_count, 1);
    assert_eq!(tree[1].path, "reading");
}

#[test]
fn sorts_siblings_and_handles_empty() {
    let tree = build_tag_tree(&[vec!["zebra".into(), "apple".into()]]);
    assert_eq!(tree[0].path, "apple");
    assert!(build_tag_tree(&[]).is_empty());
}

#[test]
fn subtree_matching_is_segment_aware() {
    assert!(tag_in_subtree("project/crane", "project"));
    assert!(tag_in_subtree("project", "project"));
    assert!(!tag_in_subtree("projectx", "project"));
    assert!(!tag_in_subtree("other", "project"));
}
