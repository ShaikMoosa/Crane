use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownDocument {
    pub blocks: Vec<MarkdownBlock>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownBlock {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Quote(Vec<Inline>),
    Code {
        language: Option<String>,
        text: String,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    Rule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub content: Vec<Inline>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    WikiLink(crate::knowledge::WikiLink),
    Text(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Code(String),
    Link { label: Vec<Inline>, url: String },
    ImageAlt(String),
}

#[derive(Clone, Copy)]
enum BlockKind {
    Paragraph,
    Heading(u8),
    Quote,
    Item,
}

struct BlockBuilder {
    kind: BlockKind,
    content: Vec<Inline>,
    checked: Option<bool>,
}

struct ListBuilder {
    ordered: bool,
    items: Vec<ListItem>,
}

enum InlineKind {
    Emphasis,
    Strong,
    Link(Option<String>),
    Image,
}

struct InlineFrame {
    kind: InlineKind,
    before: Vec<Inline>,
    children: Vec<Inline>,
}

pub fn parse_preview(markdown: &str) -> MarkdownDocument {
    let wiki_links = crate::knowledge::parse_wiki_links(markdown);
    let mut wiki_next = 0usize;
    let mut pending_text = String::new();
    let parser = Parser::new_ext(markdown, Options::ENABLE_TASKLISTS);
    let mut blocks = Vec::new();
    let mut block: Option<BlockBuilder> = None;
    let mut list: Option<ListBuilder> = None;
    let mut quote_depth = 0usize;
    let mut code: Option<(Option<String>, String)> = None;
    let mut frames: Vec<InlineFrame> = Vec::new();

    for event in parser {
        if !matches!(event, Event::Text(_)) && !pending_text.is_empty() {
            let text = std::mem::take(&mut pending_text);
            push_text_with_wiki(
                &mut block,
                &mut frames,
                text,
                markdown,
                &wiki_links,
                &mut wiki_next,
            );
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    if block.is_none() {
                        block = Some(BlockBuilder {
                            kind: if quote_depth > 0 {
                                BlockKind::Quote
                            } else {
                                BlockKind::Paragraph
                            },
                            content: Vec::new(),
                            checked: None,
                        });
                    }
                }
                Tag::Heading { level, .. } => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    block = Some(BlockBuilder {
                        kind: BlockKind::Heading(heading_level(level)),
                        content: Vec::new(),
                        checked: None,
                    });
                }
                Tag::BlockQuote(_) => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    quote_depth += 1;
                }
                Tag::CodeBlock(kind) => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    let language = match kind {
                        CodeBlockKind::Indented => None,
                        CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .filter(|value| !value.is_empty())
                            .map(str::to_string),
                    };
                    code = Some((language, String::new()));
                }
                Tag::List(start) => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    list = Some(ListBuilder {
                        ordered: start.is_some(),
                        items: Vec::new(),
                    });
                }
                Tag::Item => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    block = Some(BlockBuilder {
                        kind: BlockKind::Item,
                        content: Vec::new(),
                        checked: None,
                    });
                }
                Tag::Emphasis => start_inline(InlineKind::Emphasis, &mut block, &mut frames),
                Tag::Strong => start_inline(InlineKind::Strong, &mut block, &mut frames),
                Tag::Link { dest_url, .. } => {
                    let url = dest_url.to_string();
                    let safe = is_safe_link(&url).then_some(url);
                    start_inline(InlineKind::Link(safe), &mut block, &mut frames);
                }
                Tag::Image { .. } => start_inline(InlineKind::Image, &mut block, &mut frames),
                Tag::HtmlBlock if block.is_none() => {
                    block = Some(BlockBuilder {
                        kind: if quote_depth > 0 {
                            BlockKind::Quote
                        } else {
                            BlockKind::Paragraph
                        },
                        content: Vec::new(),
                        checked: None,
                    });
                }
                Tag::HtmlBlock => {}
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if !matches!(
                        block.as_ref().map(|value| value.kind),
                        Some(BlockKind::Item)
                    ) {
                        finish_block(&mut block, &mut list, &mut blocks);
                    }
                }
                TagEnd::Heading(_) | TagEnd::HtmlBlock => {
                    finish_block(&mut block, &mut list, &mut blocks);
                }
                TagEnd::BlockQuote(_) => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    quote_depth = quote_depth.saturating_sub(1);
                }
                TagEnd::CodeBlock => {
                    if let Some((language, text)) = code.take() {
                        blocks.push(MarkdownBlock::Code { language, text });
                    }
                }
                TagEnd::Item => finish_block(&mut block, &mut list, &mut blocks),
                TagEnd::List(_) => {
                    finish_block(&mut block, &mut list, &mut blocks);
                    if let Some(list) = list.take() {
                        blocks.push(MarkdownBlock::List {
                            ordered: list.ordered,
                            items: list.items,
                        });
                    }
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link | TagEnd::Image => {
                    finish_inline(&mut block, &mut frames);
                }
                _ => {}
            },
            Event::Text(value) => {
                if let Some((_, text)) = &mut code {
                    text.push_str(&value);
                } else {
                    pending_text.push_str(&value);
                }
            }
            Event::Code(value) => {
                push_inline(&mut block, &mut frames, Inline::Code(value.into_string()));
            }
            Event::Html(value) | Event::InlineHtml(value) => {
                push_inline(
                    &mut block,
                    &mut frames,
                    Inline::Text(neutralize_unsafe_schemes(&value)),
                );
            }
            Event::SoftBreak | Event::HardBreak => {
                push_inline(&mut block, &mut frames, Inline::Text(" ".into()));
            }
            Event::Rule => {
                finish_block(&mut block, &mut list, &mut blocks);
                blocks.push(MarkdownBlock::Rule);
            }
            Event::TaskListMarker(checked) => {
                if let Some(block) = &mut block {
                    block.checked = Some(checked);
                }
            }
            Event::InlineMath(value)
            | Event::DisplayMath(value)
            | Event::FootnoteReference(value) => {
                push_inline(&mut block, &mut frames, Inline::Text(value.into_string()));
            }
        }
    }
    if !pending_text.is_empty() {
        let text = std::mem::take(&mut pending_text);
        push_text_with_wiki(
            &mut block,
            &mut frames,
            text,
            markdown,
            &wiki_links,
            &mut wiki_next,
        );
    }
    while !frames.is_empty() {
        finish_inline(&mut block, &mut frames);
    }
    finish_block(&mut block, &mut list, &mut blocks);
    if let Some(list) = list {
        blocks.push(MarkdownBlock::List {
            ordered: list.ordered,
            items: list.items,
        });
    }
    if let Some((language, text)) = code {
        blocks.push(MarkdownBlock::Code { language, text });
    }
    MarkdownDocument { blocks }
}

fn start_inline(kind: InlineKind, block: &mut Option<BlockBuilder>, frames: &mut Vec<InlineFrame>) {
    ensure_block(block);
    let before = if let Some(frame) = frames.last_mut() {
        std::mem::take(&mut frame.children)
    } else {
        std::mem::take(&mut block.as_mut().expect("block exists").content)
    };
    frames.push(InlineFrame {
        kind,
        before,
        children: Vec::new(),
    });
}

fn finish_inline(block: &mut Option<BlockBuilder>, frames: &mut Vec<InlineFrame>) {
    let Some(frame) = frames.pop() else {
        return;
    };
    let wrapped = match frame.kind {
        InlineKind::Emphasis => Inline::Emphasis(frame.children),
        InlineKind::Strong => Inline::Strong(frame.children),
        InlineKind::Link(Some(url)) => Inline::Link {
            label: frame.children,
            url,
        },
        InlineKind::Link(None) => {
            append_many(block, frames, frame.before);
            append_many(block, frames, frame.children);
            return;
        }
        InlineKind::Image => Inline::ImageAlt(inline_plain_text(&frame.children)),
    };
    append_many(block, frames, frame.before);
    push_inline(block, frames, wrapped);
}

fn append_many(block: &mut Option<BlockBuilder>, frames: &mut [InlineFrame], values: Vec<Inline>) {
    for value in values {
        push_inline(block, frames, value);
    }
}

fn push_inline(block: &mut Option<BlockBuilder>, frames: &mut [InlineFrame], value: Inline) {
    ensure_block(block);
    if let Some(frame) = frames.last_mut() {
        frame.children.push(value);
    } else {
        block.as_mut().expect("block exists").content.push(value);
    }
}

fn ensure_block(block: &mut Option<BlockBuilder>) {
    if block.is_none() {
        *block = Some(BlockBuilder {
            kind: BlockKind::Paragraph,
            content: Vec::new(),
            checked: None,
        });
    }
}

fn finish_block(
    block: &mut Option<BlockBuilder>,
    list: &mut Option<ListBuilder>,
    blocks: &mut Vec<MarkdownBlock>,
) {
    let Some(block) = block.take() else {
        return;
    };
    match block.kind {
        BlockKind::Paragraph => blocks.push(MarkdownBlock::Paragraph(block.content)),
        BlockKind::Heading(level) => blocks.push(MarkdownBlock::Heading {
            level,
            content: block.content,
        }),
        BlockKind::Quote => blocks.push(MarkdownBlock::Quote(block.content)),
        BlockKind::Item => {
            if let Some(list) = list {
                list.items.push(ListItem {
                    checked: block.checked,
                    content: block.content,
                });
            }
        }
    }
}

fn is_safe_link(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

fn neutralize_unsafe_schemes(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut offset = 0;
    while offset < value.len() {
        let remaining = &lower[offset..];
        let blocked = ["javascript:", "data:", "file:"]
            .into_iter()
            .find(|scheme| remaining.starts_with(scheme));
        if let Some(scheme) = blocked {
            output.push_str("blocked:");
            offset += scheme.len();
        } else {
            let character = value[offset..].chars().next().expect("offset is in bounds");
            output.push(character);
            offset += character.len_utf8();
        }
    }
    output
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn inline_plain_text(inlines: &[Inline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) | Inline::ImageAlt(value) => {
                output.push_str(value)
            }
            Inline::Emphasis(children) | Inline::Strong(children) => {
                output.push_str(&inline_plain_text(children));
            }
            Inline::Link { label, .. } => output.push_str(&inline_plain_text(label)),
            Inline::WikiLink(link) => output.push_str(&link.display_text),
        }
    }
    output
}

fn push_text_with_wiki(
    block: &mut Option<BlockBuilder>,
    frames: &mut Vec<InlineFrame>,
    text: String,
    source: &str,
    links: &[crate::knowledge::WikiLink],
    next: &mut usize,
) {
    let mut rest = text.as_str();
    loop {
        if *next >= links.len() {
            if !rest.is_empty() {
                push_inline(block, frames, Inline::Text(rest.to_string()));
            }
            return;
        }
        let link = &links[*next];
        let needle = &source[link.range.clone()];
        match rest.find(needle) {
            Some(pos) => {
                if pos > 0 {
                    push_inline(block, frames, Inline::Text(rest[..pos].to_string()));
                }
                push_inline(block, frames, Inline::WikiLink(link.clone()));
                *next += 1;
                rest = &rest[pos + needle.len()..];
                if rest.is_empty() {
                    return;
                }
            }
            None => {
                if !rest.is_empty() {
                    push_inline(block, frames, Inline::Text(rest.to_string()));
                }
                return;
            }
        }
    }
}
