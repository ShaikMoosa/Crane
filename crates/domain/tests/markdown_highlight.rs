use crane_domain::{highlight_markdown, MarkdownRole, MarkdownSpan};
use std::ops::Range;

fn assert_invariants(source: &str, spans: &[MarkdownSpan]) {
    if source.is_empty() {
        assert!(spans.is_empty());
        return;
    }

    assert_eq!(spans.first().unwrap().byte_range.start, 0);
    assert_eq!(spans.last().unwrap().byte_range.end, source.len());
    for span in spans {
        assert!(
            span.byte_range.start < span.byte_range.end,
            "empty span: {span:?}"
        );
        assert!(source.is_char_boundary(span.byte_range.start));
        assert!(source.is_char_boundary(span.byte_range.end));
    }
    for pair in spans.windows(2) {
        assert_eq!(
            pair[0].byte_range.end, pair[1].byte_range.start,
            "spans were not adjacent: {pair:?}"
        );
    }
}

fn assert_role(source: &str, spans: &[MarkdownSpan], range: Range<usize>, role: MarkdownRole) {
    let span = spans
        .iter()
        .find(|span| span.byte_range.start <= range.start && span.byte_range.end >= range.end)
        .unwrap_or_else(|| {
            panic!(
                "no span covers {:?} ({:?}) in {spans:#?}",
                range,
                &source[range.clone()]
            )
        });
    assert_eq!(span.role, role, "wrong role for {:?}", &source[range]);
}

fn range_of(source: &str, needle: &str) -> Range<usize> {
    let start = source.find(needle).expect("needle exists");
    start..start + needle.len()
}

#[test]
fn atx_headings_one_through_six_keep_markers_and_style_content() {
    for level in 1_u8..=6 {
        let marker = "#".repeat(level.into());
        let source = format!("{marker} Heading {level}\r\nbody");
        let spans = highlight_markdown(&source);
        assert_invariants(&source, &spans);
        assert_role(&source, &spans, 0..marker.len(), MarkdownRole::Marker);
        let heading = format!("Heading {level}");
        assert_role(
            &source,
            &spans,
            range_of(&source, &heading),
            MarkdownRole::Heading(level),
        );
    }
}

#[test]
fn inline_roles_links_wiki_tags_and_nested_markup_have_exact_source_ranges() {
    let source = "**bold _soft_** ~~gone~~ [Crane](https://example.com) [[Target#Part|Alias]] #work/process `#code [[No]]`";
    let unchanged = source.to_owned();
    let spans = highlight_markdown(source);
    assert_invariants(source, &spans);
    assert_eq!(source, unchanged);

    assert_role(source, &spans, 0..2, MarkdownRole::Marker);
    assert_role(
        source,
        &spans,
        range_of(source, "bold "),
        MarkdownRole::Strong,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "soft"),
        MarkdownRole::Emphasis,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "gone"),
        MarkdownRole::Strikethrough,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "Crane"),
        MarkdownRole::LinkText,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "https://example.com"),
        MarkdownRole::LinkDestination,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "[[Target#Part|Alias]]"),
        MarkdownRole::WikiLink,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "#work/process"),
        MarkdownRole::Tag,
    );
    assert_role(
        source,
        &spans,
        range_of(source, "`#code [[No]]`"),
        MarkdownRole::InlineCode,
    );
}

#[test]
fn block_and_line_markers_obey_code_precedence() {
    let source = "```md\r\n#not-a-tag [[No]]\r\n```\r\n> - [x] done\r\n1. next #tag [[Yes]]";
    let spans = highlight_markdown(source);
    assert_invariants(source, &spans);

    let block = 0..source.find("\r\n> ").unwrap() + 2;
    assert_role(source, &spans, block, MarkdownRole::CodeBlock);
    assert_role(
        source,
        &spans,
        range_of(source, ">"),
        MarkdownRole::QuoteMarker,
    );
    let bullet = source.find("\r\n> -").unwrap() + "\r\n> ".len();
    assert_role(source, &spans, bullet..bullet + 1, MarkdownRole::ListMarker);
    assert_role(
        source,
        &spans,
        range_of(source, "[x]"),
        MarkdownRole::TaskMarker { checked: true },
    );
    assert_role(
        source,
        &spans,
        range_of(source, "1."),
        MarkdownRole::ListMarker,
    );
    assert_role(source, &spans, range_of(source, "#tag"), MarkdownRole::Tag);
    assert_role(
        source,
        &spans,
        range_of(source, "[[Yes]]"),
        MarkdownRole::WikiLink,
    );
}

#[test]
fn unicode_crlf_and_malformed_input_are_total_and_boundary_safe() {
    let cases = [
        "# शीर्षक 世界 👋🏽 e\u{301}\r\n**強い** #タグ",
        "",
        "`unclosed देवनागरी",
        "```rust\r\nfn main() { println!(\"世界\"); }",
        "[[unfinished",
        "[unclosed",
        "**unclosed",
        "~~unclosed",
        r"\*escaped\* \#not-a-tag",
        "emoji 👨‍👩‍👧‍👦 and combining e\u{301}",
    ];

    for source in cases {
        let spans = highlight_markdown(source);
        assert_invariants(source, &spans);
    }

    let escaped = r"\*escaped\* \#not-a-tag";
    let spans = highlight_markdown(escaped);
    assert!(spans.iter().all(|span| span.role == MarkdownRole::Plain));

    let unclosed_tick = "`unclosed";
    let spans = highlight_markdown(unclosed_tick);
    assert_role(unclosed_tick, &spans, 0..1, MarkdownRole::Marker);
    assert_role(
        unclosed_tick,
        &spans,
        1..unclosed_tick.len(),
        MarkdownRole::Plain,
    );
}

#[test]
fn plain_text_is_one_gap_free_span_and_empty_input_has_none() {
    let source = "ordinary text";
    let spans = highlight_markdown(source);
    assert_eq!(
        spans,
        vec![MarkdownSpan {
            byte_range: 0..source.len(),
            role: MarkdownRole::Plain,
        }]
    );
    assert!(highlight_markdown("").is_empty());
}
