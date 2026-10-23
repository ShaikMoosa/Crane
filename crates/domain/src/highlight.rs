use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownSpan {
    pub byte_range: Range<usize>,
    pub role: MarkdownRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkdownRole {
    Plain,
    Marker,
    Heading(u8),
    Strong,
    Emphasis,
    Strikethrough,
    LinkText,
    LinkDestination,
    WikiLink,
    Tag,
    QuoteMarker,
    ListMarker,
    TaskMarker { checked: bool },
    InlineCode,
    CodeBlock,
}

#[derive(Clone, Debug)]
struct Candidate {
    byte_range: Range<usize>,
    role: MarkdownRole,
    priority: u8,
    order: usize,
}

const PRIORITY_CONTENT: u8 = 40;
const PRIORITY_INLINE: u8 = 50;
const PRIORITY_MARKER: u8 = 60;
const PRIORITY_STRUCTURAL_MARKER: u8 = 70;
const PRIORITY_LINK_TARGET: u8 = 80;
const PRIORITY_INLINE_CODE: u8 = 90;
const PRIORITY_CODE_BLOCK: u8 = 100;

pub fn highlight_markdown(source: &str) -> Vec<MarkdownSpan> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let code_blocks = code_block_ranges(source);
    for range in &code_blocks {
        add_candidate(
            source,
            &mut candidates,
            range.clone(),
            MarkdownRole::CodeBlock,
            PRIORITY_CODE_BLOCK,
        );
    }
    scan_pulldown(source, &mut candidates);
    scan_inline_code(source, &code_blocks, &mut candidates);
    scan_line_markers(source, &code_blocks, &mut candidates);
    scan_tags(source, &mut candidates);
    for link in crate::knowledge::parse_wiki_links(source) {
        add_candidate(
            source,
            &mut candidates,
            link.range,
            MarkdownRole::WikiLink,
            PRIORITY_LINK_TARGET,
        );
    }
    scan_incomplete_markers(source, &code_blocks, &mut candidates);
    compose(source, candidates)
}

fn add_candidate(
    source: &str,
    candidates: &mut Vec<Candidate>,
    byte_range: Range<usize>,
    role: MarkdownRole,
    priority: u8,
) {
    if byte_range.start >= byte_range.end
        || byte_range.end > source.len()
        || !source.is_char_boundary(byte_range.start)
        || !source.is_char_boundary(byte_range.end)
    {
        return;
    }
    let order = candidates.len();
    candidates.push(Candidate {
        byte_range,
        role,
        priority,
        order,
    });
}

fn compose(source: &str, candidates: Vec<Candidate>) -> Vec<MarkdownSpan> {
    let mut boundaries = Vec::with_capacity(candidates.len() * 2 + 2);
    boundaries.push(0);
    boundaries.push(source.len());
    for candidate in &candidates {
        boundaries.push(candidate.byte_range.start);
        boundaries.push(candidate.byte_range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut spans: Vec<MarkdownSpan> = Vec::new();
    for pair in boundaries.windows(2) {
        let range = pair[0]..pair[1];
        if range.is_empty() {
            continue;
        }
        let role = candidates
            .iter()
            .filter(|candidate| {
                candidate.byte_range.start <= range.start && candidate.byte_range.end >= range.end
            })
            .max_by(|left, right| {
                left.priority
                    .cmp(&right.priority)
                    .then_with(|| {
                        let left_len = left.byte_range.end - left.byte_range.start;
                        let right_len = right.byte_range.end - right.byte_range.start;
                        right_len.cmp(&left_len)
                    })
                    .then_with(|| left.order.cmp(&right.order))
            })
            .map_or(MarkdownRole::Plain, |candidate| candidate.role.clone());

        if let Some(previous) = spans.last_mut() {
            if previous.role == role && previous.byte_range.end == range.start {
                previous.byte_range.end = range.end;
                continue;
            }
        }
        spans.push(MarkdownSpan {
            byte_range: range,
            role,
        });
    }
    spans
}

#[derive(Clone, Debug)]
enum ActiveRole {
    Heading(u8),
    Strong,
    Emphasis,
    Strikethrough,
    Link,
}

impl ActiveRole {
    fn markdown_role(&self) -> MarkdownRole {
        match self {
            Self::Heading(level) => MarkdownRole::Heading(*level),
            Self::Strong => MarkdownRole::Strong,
            Self::Emphasis => MarkdownRole::Emphasis,
            Self::Strikethrough => MarkdownRole::Strikethrough,
            Self::Link => MarkdownRole::LinkText,
        }
    }

    fn priority(&self) -> u8 {
        match self {
            Self::Heading(_) => PRIORITY_CONTENT,
            Self::Strong | Self::Emphasis | Self::Strikethrough | Self::Link => PRIORITY_INLINE,
        }
    }
}

fn scan_pulldown(source: &str, candidates: &mut Vec<Candidate>) {
    let options = Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let mut active = Vec::new();
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    active.push(ActiveRole::Heading(heading_level(level)));
                }
                Tag::Strong => {
                    mark_container_delimiters(source, candidates, range, 2, MarkdownRole::Marker);
                    active.push(ActiveRole::Strong);
                }
                Tag::Emphasis => {
                    mark_container_delimiters(source, candidates, range, 1, MarkdownRole::Marker);
                    active.push(ActiveRole::Emphasis);
                }
                Tag::Strikethrough => {
                    mark_container_delimiters(source, candidates, range, 2, MarkdownRole::Marker);
                    active.push(ActiveRole::Strikethrough);
                }
                Tag::Link { .. } => {
                    scan_inline_link(source, range, candidates);
                    active.push(ActiveRole::Link);
                }
                _ => {}
            },
            Event::End(tag) => {
                if matches!(
                    tag,
                    TagEnd::Heading(_)
                        | TagEnd::Strong
                        | TagEnd::Emphasis
                        | TagEnd::Strikethrough
                        | TagEnd::Link
                ) {
                    active.pop();
                }
            }
            Event::Text(_) => {
                if let Some(role) = active.last() {
                    add_candidate(
                        source,
                        candidates,
                        range,
                        role.markdown_role(),
                        role.priority(),
                    );
                }
            }
            Event::Code(_) => add_candidate(
                source,
                candidates,
                range,
                MarkdownRole::InlineCode,
                PRIORITY_INLINE_CODE,
            ),
            Event::TaskListMarker(checked) => add_candidate(
                source,
                candidates,
                range,
                MarkdownRole::TaskMarker { checked },
                PRIORITY_STRUCTURAL_MARKER,
            ),
            _ => {}
        }
    }
}

fn mark_container_delimiters(
    source: &str,
    candidates: &mut Vec<Candidate>,
    range: Range<usize>,
    delimiter_len: usize,
    role: MarkdownRole,
) {
    if range.end.saturating_sub(range.start) < delimiter_len * 2 {
        return;
    }
    add_candidate(
        source,
        candidates,
        range.start..range.start + delimiter_len,
        role.clone(),
        PRIORITY_MARKER,
    );
    add_candidate(
        source,
        candidates,
        range.end - delimiter_len..range.end,
        role,
        PRIORITY_MARKER,
    );
}

fn scan_inline_link(source: &str, range: Range<usize>, candidates: &mut Vec<Candidate>) {
    let raw = &source[range.clone()];
    let Some(open) = raw.find('[') else {
        return;
    };
    let label_start = range.start + open + 1;
    let Some(close_relative) = raw[open + 1..].find("](") else {
        return;
    };
    let label_end = range.start + open + 1 + close_relative;
    let destination_start = label_end + 2;
    let Some(close_paren_relative) = source[destination_start..range.end].rfind(')') else {
        return;
    };
    let destination_end = destination_start + close_paren_relative;

    add_candidate(
        source,
        candidates,
        range.start + open..label_start,
        MarkdownRole::Marker,
        PRIORITY_MARKER,
    );
    add_candidate(
        source,
        candidates,
        label_start..label_end,
        MarkdownRole::LinkText,
        PRIORITY_INLINE,
    );
    add_candidate(
        source,
        candidates,
        label_end..destination_start,
        MarkdownRole::Marker,
        PRIORITY_MARKER,
    );
    add_candidate(
        source,
        candidates,
        destination_start..destination_end,
        MarkdownRole::LinkDestination,
        PRIORITY_LINK_TARGET,
    );
    add_candidate(
        source,
        candidates,
        destination_end..destination_end + 1,
        MarkdownRole::Marker,
        PRIORITY_MARKER,
    );
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

fn code_block_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut offset = 0usize;
    let mut fence: Option<(usize, u8, usize)> = None;
    for part in source.split_inclusive('\n') {
        let start = offset;
        offset += part.len();
        let line = part.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim_start_matches([' ', '\t']);
        let leading = line.len() - trimmed.len();
        if let Some((block_start, delimiter, minimum)) = fence {
            if is_closing_fence(trimmed, leading, delimiter, minimum) {
                ranges.push(block_start..offset);
                fence = None;
            }
            continue;
        }
        if let Some((delimiter, length)) = opening_fence(trimmed, leading) {
            fence = Some((start, delimiter, length));
        } else if line.starts_with("    ") || line.starts_with('\t') {
            ranges.push(start..offset);
        }
    }
    if let Some((start, _, _)) = fence {
        ranges.push(start..source.len());
    }
    ranges
}

fn opening_fence(trimmed: &str, leading: usize) -> Option<(u8, usize)> {
    if leading > 3 {
        return None;
    }
    let delimiter = *trimmed.as_bytes().first()?;
    if delimiter != b'`' && delimiter != b'~' {
        return None;
    }
    let length = trimmed
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == delimiter)
        .count();
    (length >= 3).then_some((delimiter, length))
}

fn is_closing_fence(trimmed: &str, leading: usize, delimiter: u8, minimum: usize) -> bool {
    if leading > 3 {
        return false;
    }
    let run = trimmed
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == delimiter)
        .count();
    run >= minimum && trimmed[run..].trim_matches([' ', '\t']).is_empty()
}

fn scan_inline_code(source: &str, code_blocks: &[Range<usize>], candidates: &mut Vec<Candidate>) {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if let Some(block) = containing_range(code_blocks, cursor) {
            cursor = block.end;
            continue;
        }
        if bytes[cursor] != b'`' || is_escaped(source, cursor) {
            cursor += 1;
            continue;
        }
        let run = bytes[cursor..]
            .iter()
            .take_while(|byte| **byte == b'`')
            .count();
        let mut search = cursor + run;
        let mut close = None;
        while search < bytes.len() {
            if let Some(block) = containing_range(code_blocks, search) {
                search = block.end;
                continue;
            }
            if bytes[search] == b'`' && !is_escaped(source, search) {
                let close_run = bytes[search..]
                    .iter()
                    .take_while(|byte| **byte == b'`')
                    .count();
                if close_run == run {
                    close = Some(search + close_run);
                    break;
                }
                search += close_run;
            } else {
                search += 1;
            }
        }
        if let Some(end) = close {
            add_candidate(
                source,
                candidates,
                cursor..end,
                MarkdownRole::InlineCode,
                PRIORITY_INLINE_CODE,
            );
            cursor = end;
        } else {
            add_candidate(
                source,
                candidates,
                cursor..cursor + run,
                MarkdownRole::Marker,
                PRIORITY_MARKER,
            );
            cursor += run;
        }
    }
}

fn scan_line_markers(source: &str, code_blocks: &[Range<usize>], candidates: &mut Vec<Candidate>) {
    let mut offset = 0usize;
    for part in source.split_inclusive('\n') {
        let line_start = offset;
        offset += part.len();
        if containing_range(code_blocks, line_start).is_some() {
            continue;
        }
        let line = part.trim_end_matches(['\r', '\n']);
        let bytes = line.as_bytes();
        let mut cursor = bytes
            .iter()
            .take_while(|byte| **byte == b' ' || **byte == b'\t')
            .count();
        if cursor > 3 {
            continue;
        }

        while cursor < bytes.len() && bytes[cursor] == b'>' {
            add_candidate(
                source,
                candidates,
                line_start + cursor..line_start + cursor + 1,
                MarkdownRole::QuoteMarker,
                PRIORITY_STRUCTURAL_MARKER,
            );
            cursor += 1;
            while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t') {
                cursor += 1;
            }
        }

        let hashes = bytes[cursor..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        if (1..=6).contains(&hashes)
            && (cursor + hashes == bytes.len() || matches!(bytes[cursor + hashes], b' ' | b'\t'))
        {
            add_candidate(
                source,
                candidates,
                line_start + cursor..line_start + cursor + hashes,
                MarkdownRole::Marker,
                PRIORITY_STRUCTURAL_MARKER,
            );
            continue;
        }

        let marker_len = list_marker_len(&bytes[cursor..]);
        if marker_len == 0 {
            continue;
        }
        add_candidate(
            source,
            candidates,
            line_start + cursor..line_start + cursor + marker_len,
            MarkdownRole::ListMarker,
            PRIORITY_STRUCTURAL_MARKER,
        );
        cursor += marker_len;
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t') {
            cursor += 1;
        }
        if cursor + 3 <= bytes.len()
            && bytes[cursor] == b'['
            && bytes[cursor + 2] == b']'
            && matches!(bytes[cursor + 1], b' ' | b'x' | b'X')
        {
            add_candidate(
                source,
                candidates,
                line_start + cursor..line_start + cursor + 3,
                MarkdownRole::TaskMarker {
                    checked: !matches!(bytes[cursor + 1], b' '),
                },
                PRIORITY_STRUCTURAL_MARKER,
            );
        }
    }
}

fn list_marker_len(bytes: &[u8]) -> usize {
    if bytes.len() >= 2 && matches!(bytes[0], b'-' | b'+' | b'*') && bytes[1].is_ascii_whitespace()
    {
        return 1;
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > 0
        && digits + 1 < bytes.len()
        && matches!(bytes[digits], b'.' | b')')
        && bytes[digits + 1].is_ascii_whitespace()
    {
        digits + 1
    } else {
        0
    }
}

fn scan_tags(source: &str, candidates: &mut Vec<Candidate>) {
    let recognized: HashSet<String> = crate::extract_tags(source).into_iter().collect();
    if recognized.is_empty() {
        return;
    }
    let mut cursor = 0usize;
    while cursor < source.len() {
        let character = source[cursor..].chars().next().expect("cursor in bounds");
        if character.is_whitespace() {
            cursor += character.len_utf8();
            continue;
        }
        let start = cursor;
        while cursor < source.len() {
            let character = source[cursor..].chars().next().expect("cursor in bounds");
            if character.is_whitespace() {
                break;
            }
            cursor += character.len_utf8();
        }
        let token = &source[start..cursor];
        if !token.starts_with('#') || is_escaped(source, start) {
            continue;
        }
        let raw_len = token[1..]
            .char_indices()
            .take_while(|(_, ch)| ch.is_alphanumeric() || matches!(ch, '_' | '-' | '/'))
            .last()
            .map_or(0, |(at, ch)| at + ch.len_utf8());
        if raw_len == 0 {
            continue;
        }
        let raw = &token[1..1 + raw_len];
        if raw.split('/').any(str::is_empty) || !recognized.contains(&raw.to_lowercase()) {
            continue;
        }
        add_candidate(
            source,
            candidates,
            start..start + 1 + raw_len,
            MarkdownRole::Tag,
            PRIORITY_INLINE,
        );
    }
}

fn scan_incomplete_markers(
    source: &str,
    code_blocks: &[Range<usize>],
    candidates: &mut Vec<Candidate>,
) {
    let mut cursor = 0usize;
    while let Some(found) = source[cursor..].find("[[") {
        let start = cursor + found;
        if containing_range(code_blocks, start).is_none()
            && !is_escaped(source, start)
            && !source[start + 2..]
                .split(['\r', '\n'])
                .next()
                .is_some_and(|line| line.contains("]]"))
        {
            add_candidate(
                source,
                candidates,
                start..start + 2,
                MarkdownRole::Marker,
                PRIORITY_MARKER,
            );
        }
        cursor = start + 2;
    }
}

fn containing_range(ranges: &[Range<usize>], position: usize) -> Option<&Range<usize>> {
    ranges
        .iter()
        .find(|range| position >= range.start && position < range.end)
}

fn is_escaped(source: &str, position: usize) -> bool {
    let mut slash_count = 0usize;
    let mut cursor = position;
    while cursor > 0 && source.as_bytes()[cursor - 1] == b'\\' {
        slash_count += 1;
        cursor -= 1;
    }
    slash_count % 2 == 1
}
