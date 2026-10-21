use crane_domain::knowledge::{
    normalize_heading_anchor, normalize_title, parse_headings, parse_wiki_links,
};

#[test]
fn parses_alias_and_heading_with_byte_range() {
    let body = "See [[Field Notes#On the desk|my notes]] end";
    let links = parse_wiki_links(body);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_title, "Field Notes");
    assert_eq!(links[0].heading.as_ref().unwrap().raw, "On the desk");
    assert_eq!(links[0].display_text, "my notes");
    assert_eq!(
        &body[links[0].range.clone()],
        "[[Field Notes#On the desk|my notes]]"
    );
}

#[test]
fn parses_all_spec_forms() {
    let links = parse_wiki_links("[[A]] [[B|bee]] [[C#Sec]] [[D#Sec|dee]]");
    assert_eq!(links.len(), 4);
    assert_eq!(links[0].display_text, "A");
    assert!(links[0].heading.is_none() && links[0].alias.is_none());
    assert_eq!(links[1].alias.as_deref(), Some("bee"));
    assert_eq!(links[2].heading.as_ref().unwrap().raw, "Sec");
    assert_eq!(links[3].display_text, "dee");
    // Empty heading means no heading; empty alias falls back to the title.
    let links = parse_wiki_links("[[E#]] [[F|]] [[ G # H | ]]");
    assert_eq!(links.len(), 3);
    assert!(links[0].heading.is_none());
    assert_eq!(links[1].display_text, "F");
    assert_eq!(links[2].target_title, "G");
    assert_eq!(links[2].heading.as_ref().unwrap().raw, "H");
    assert_eq!(links[2].display_text, "G");
}

#[test]
fn escaped_and_malformed_forms_stay_text() {
    assert!(parse_wiki_links(r"\[[Not a link]]").is_empty());
    assert!(parse_wiki_links("[[ ]]").is_empty());
    assert!(parse_wiki_links("[[Unclosed").is_empty());
    assert!(parse_wiki_links("[[A [[B]]]]").is_empty());
    assert!(parse_wiki_links("[[A\n]]").is_empty());
    // An escaped backslash does not escape the bracket: this is a link.
    let links = parse_wiki_links(r"\\[[Real]]");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_title, "Real");
    // Escaped separators live inside the title.
    let links = parse_wiki_links(r"[[A\|B]]");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_title, "A|B");
}

#[test]
fn code_spans_and_fences_hide_links() {
    let body = "`[[Inline]]` text [[Visible]]\n\n```\n[[Fenced]]\n```\n\n    [[Indented]]\n\n~~~\n[[Tilde]]\n~~~\n";
    let links = parse_wiki_links(body);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_title, "Visible");
}

#[test]
fn headings_carry_levels_text_anchors_and_ranges() {
    let body = "# Title\n\n## On the desk\n\n## On the desk\n\n## Intro-2\n\n####### Not a heading\n";
    let headings = parse_headings(body);
    assert_eq!(headings.len(), 4);
    assert_eq!(headings[0].level, 1);
    assert_eq!(headings[1].text, "On the desk");
    assert_eq!(headings[1].anchor, "on-the-desk");
    assert_eq!(headings[2].anchor, "on-the-desk-2");
    assert_eq!(headings[3].anchor, "intro-2");
    assert_eq!(&body[headings[1].range.clone()], "On the desk");
}

#[test]
fn duplicate_anchor_collision_with_literal_suffix() {
    let headings = parse_headings("## Intro\n\n## Intro\n\n## Intro-2\n");
    let anchors: Vec<_> = headings.iter().map(|h| h.anchor.as_str()).collect();
    assert_eq!(anchors, ["intro", "intro-2", "intro-2-2"]);
}

#[test]
fn normalization_keeps_punctuation_and_uses_unicode_lowercase() {
    assert_eq!(normalize_title("  ÀÉÎ   Note:  Draft "), "àéî note: draft");
    assert_eq!(normalize_heading_anchor("## On  the Desk!"), "on-the-desk!");
    let headings = parse_headings("```\n# Hidden\n```\n\n# Shown\n");
    assert_eq!(headings.len(), 1);
    assert_eq!(headings[0].text, "Shown");
}

#[test]
fn parser_is_total_and_does_not_modify_source() {
    for body in ["", "[[", "]]", "\\", "\u{10FFFF}\u{301}", "[[é]]\r\n[[x]]"] {
        let before = body.to_string();
        let _ = parse_wiki_links(body);
        let _ = parse_headings(body);
        assert_eq!(body, before);
    }
    let links = parse_wiki_links("[[é]]\r\n[[x]]");
    assert_eq!(links.len(), 2);
    assert_eq!(links[1].target_title, "x");
}
