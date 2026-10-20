use crane_domain::{parse_preview, Inline, MarkdownBlock};

fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) | Inline::ImageAlt(value) => {
                text.push_str(value)
            }
            Inline::Emphasis(children) | Inline::Strong(children) => {
                text.push_str(&inline_text(children))
            }
            Inline::Link { label, .. } => text.push_str(&inline_text(label)),
        }
    }
    text
}

#[test]
fn html_is_inert_and_unsafe_links_are_plain_text() {
    let document = parse_preview(
        "# Hello\n\n<script>alert(1)</script> [bad](javascript:alert(1)) [file](file:///x)",
    );
    let debug = format!("{document:?}");
    assert!(debug.contains("<script>alert(1)</script>"));
    assert!(!debug.contains("javascript:"), "{debug}");
    assert!(!debug.contains("file:///"), "{debug}");
    assert!(debug.contains("bad"));
    assert!(debug.contains("file"));
}

#[test]
fn safe_links_are_typed_but_images_keep_alt_text_only() {
    let document =
        parse_preview("[site](https://example.com) ![diagram](https://example.com/image.png)");
    let MarkdownBlock::Paragraph(inlines) = &document.blocks[0] else {
        panic!("expected paragraph")
    };
    assert!(matches!(&inlines[0], Inline::Link { url, .. } if url == "https://example.com"));
    assert!(inlines
        .iter()
        .any(|inline| matches!(inline, Inline::ImageAlt(alt) if alt == "diagram")));
    assert!(!format!("{document:?}").contains("image.png"));
}

#[test]
fn parses_nested_formatting_code_quotes_lists_and_read_only_tasks() {
    let document = parse_preview(
        "Paragraph **bold and *calm***.\n\n> Quoted\n\n```rust\nfn main() {}\n```\n\n- [x] shipped\n- ordinary",
    );
    assert!(
        matches!(&document.blocks[0], MarkdownBlock::Paragraph(inlines) if inline_text(inlines).contains("bold and calm"))
    );
    assert!(document.blocks.iter().any(
        |block| matches!(block, MarkdownBlock::Quote(inlines) if inline_text(inlines) == "Quoted")
    ));
    assert!(document.blocks.iter().any(|block| matches!(block, MarkdownBlock::Code { language: Some(language), text } if language == "rust" && text.contains("fn main"))));
    let list = document
        .blocks
        .iter()
        .find_map(|block| match block {
            MarkdownBlock::List { items, .. } => Some(items),
            _ => None,
        })
        .unwrap();
    assert_eq!(list[0].checked, Some(true));
    assert_eq!(inline_text(&list[0].content), "shipped");
    assert_eq!(list[1].checked, None);
}
