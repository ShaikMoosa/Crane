use std::collections::{HashMap, HashSet};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingFact {
    pub level: u8,
    pub text: String,
    pub normalized: String,
    pub anchor: String,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingTarget {
    pub raw: String,
    pub normalized: String,
    pub anchor_base: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLink {
    pub range: Range<usize>,
    pub target_title: String,
    pub normalized_title: String,
    pub heading: Option<HeadingTarget>,
    pub alias: Option<String>,
    pub display_text: String,
}

pub fn normalize_title(raw: &str) -> String {
    raw.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_heading_anchor(raw: &str) -> String {
    let trimmed = raw.trim().trim_start_matches('#').trim();
    let lowered = trimmed.to_lowercase();
    let mut anchor = String::with_capacity(lowered.len());
    let mut last_was_dash = true;
    for ch in lowered.chars() {
        if ch.is_whitespace() {
            if !last_was_dash {
                anchor.push('-');
                last_was_dash = true;
            }
        } else {
            anchor.push(ch);
            last_was_dash = false;
        }
    }
    while anchor.ends_with('-') {
        anchor.pop();
    }
    anchor
}

struct LineInfo {
    range: Range<usize>,
    is_code: bool,
}

fn line_infos(body: &str) -> Vec<LineInfo> {
    let mut infos = Vec::new();
    let mut offset = 0usize;
    let mut fence: Option<(char, usize)> = None;
    for part in body.split_inclusive('\n') {
        let content = part.trim_end_matches(['\r', '\n']);
        let start = offset;
        offset += part.len();
        let trimmed = content.trim_start_matches([' ', '\t']);
        let leading = content.len() - trimmed.len();
        let mut is_code = false;
        if let Some((fence_char, fence_len)) = fence {
            is_code = true;
            if let Some(run) = leading_run(trimmed, fence_char) {
                if run >= fence_len && leading <= 3 {
                    fence = None;
                }
            }
        } else if let Some((fence_char, run)) = fence_start(trimmed, leading) {
            fence = Some((fence_char, run));
            is_code = true;
        } else if content.starts_with("    ") || content.starts_with('\t') {
            is_code = true;
        }
        infos.push(LineInfo {
            range: start..offset,
            is_code,
        });
        let _ = content;
    }
    infos
}

fn leading_run(text: &str, ch: char) -> Option<usize> {
    let run = text.chars().take_while(|c| *c == ch).count();
    (run > 0).then_some(run)
}

fn fence_start(trimmed: &str, leading: usize) -> Option<(char, usize)> {
    if leading > 3 {
        return None;
    }
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let run = leading_run(trimmed, first)?;
    (run >= 3).then_some((first, run))
}

fn inline_code_ranges(line: &str, line_start: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run = bytes[i..].iter().take_while(|b| **b == b'`').count();
            // Find a closing run of exactly the same length.
            let mut j = i + run;
            let mut closed = None;
            while j < bytes.len() {
                if bytes[j] == b'`' {
                    let close_run = bytes[j..].iter().take_while(|b| **b == b'`').count();
                    if close_run == run {
                        closed = Some(j + close_run);
                        break;
                    }
                    j += close_run;
                } else {
                    j += 1;
                }
            }
            match closed {
                Some(end) => {
                    ranges.push(line_start + i..line_start + end);
                    i = end;
                }
                None => break,
            }
        } else {
            i += 1;
        }
    }
    ranges
}

fn code_ranges(body: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for info in line_infos(body) {
        if info.is_code {
            ranges.push(info.range);
        } else {
            let line = &body[info.range.clone()];
            let content = line.trim_end_matches(['\r', '\n']);
            ranges.extend(inline_code_ranges(content, info.range.start));
        }
    }
    ranges
}

fn in_ranges(ranges: &[Range<usize>], pos: usize) -> bool {
    ranges.iter().any(|r| pos >= r.start && pos < r.end)
}

fn is_escaped(body: &str, pos: usize) -> bool {
    let mut count = 0usize;
    let mut i = pos;
    while i > 0 && body.as_bytes()[i - 1] == b'\\' {
        count += 1;
        i -= 1;
    }
    count % 2 == 1
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('|') | Some('#') | Some(']') | Some('[') | Some('\\') => {
                    out.push(chars.next().unwrap());
                }
                _ => out.push(ch),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Split raw link content at the first unescaped `|` (alias) and the first
/// unescaped `#` before it (heading). Returns (title, heading, alias) raw parts.
fn split_content(content: &str) -> (String, Option<String>, Option<String>) {
    let bytes = content.as_bytes();
    let mut alias_at = None;
    let mut heading_at = None;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'|' && alias_at.is_none() {
            alias_at = Some(i);
            break;
        }
        if bytes[i] == b'#' && heading_at.is_none() {
            heading_at = Some(i);
        }
        i += 1;
    }
    let (before_alias, alias) = match alias_at {
        Some(at) => (&content[..at], Some(content[at + 1..].to_string())),
        None => (content, None),
    };
    let (title, heading) = match heading_at.filter(|at| *at <= before_alias.len()) {
        Some(at) => (
            &before_alias[..at],
            Some(before_alias[at + 1..].to_string()),
        ),
        None => (before_alias, None),
    };
    (title.to_string(), heading, alias)
}

pub fn parse_wiki_links(body: &str) -> Vec<WikiLink> {
    let code = code_ranges(body);
    let bytes = body.as_bytes();
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = body[cursor..].find("[[") {
        let start = cursor + found;
        if in_ranges(&code, start) || is_escaped(body, start) {
            cursor = start + 2;
            continue;
        }
        // Closing "]]" must be on the same line and not escaped.
        let Some(close_rel) = body[start + 2..].find("]]") else {
            break;
        };
        let close = start + 2 + close_rel;
        let content = &body[start + 2..close];
        if content.contains(['\n', '\r']) || is_escaped(body, close) {
            cursor = start + 2;
            continue;
        }
        if contains_unescaped(content, "[[") {
            // Nested opener: the whole construct is ordinary text.
            cursor = close + 2;
            continue;
        }
        let _ = bytes;
        let (title_raw, heading_raw, alias_raw) = split_content(content);
        let target_title = unescape(title_raw.trim());
        if target_title.is_empty() {
            cursor = close + 2;
            continue;
        }
        let heading = heading_raw.and_then(|raw| {
            let raw = unescape(raw.trim());
            if raw.is_empty() {
                None
            } else {
                Some(HeadingTarget {
                    normalized: normalize_title(&raw),
                    anchor_base: normalize_heading_anchor(&raw),
                    raw,
                })
            }
        });
        let alias = alias_raw.and_then(|raw| {
            let value = unescape(raw.trim());
            (!value.is_empty()).then_some(value)
        });
        let display_text = alias.clone().unwrap_or_else(|| target_title.clone());
        links.push(WikiLink {
            range: start..close + 2,
            normalized_title: normalize_title(&target_title),
            target_title,
            heading,
            alias,
            display_text,
        });
        cursor = close + 2;
    }
    links
}

fn contains_unescaped(text: &str, needle: &str) -> bool {
    let mut cursor = 0usize;
    while let Some(found) = text[cursor..].find(needle) {
        let at = cursor + found;
        if !is_escaped(text, at) {
            return true;
        }
        cursor = at + needle.len();
    }
    false
}

fn plain_heading_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '`' | '*' | '~' => {}
            '!' if chars.peek() == Some(&'[') => {
                chars.next();
                for inner in chars.by_ref() {
                    if inner == ']' {
                        break;
                    }
                    out.push(inner);
                }
                if chars.peek() == Some(&'(') {
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
            }
            '[' => {
                let mut label = String::new();
                let mut closed = false;
                for inner in chars.by_ref() {
                    if inner == ']' {
                        closed = true;
                        break;
                    }
                    label.push(inner);
                }
                if closed && chars.peek() == Some(&'(') {
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                out.push_str(&label);
            }
            _ => out.push(ch),
        }
    }
    out.trim().to_string()
}

pub fn parse_headings(body: &str) -> Vec<HeadingFact> {
    let infos = line_infos(body);
    let mut headings = Vec::new();
    let mut occurrences: HashMap<String, usize> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    for info in infos {
        if info.is_code {
            continue;
        }
        let line_full = &body[info.range.clone()];
        let line = line_full.trim_end_matches(['\r', '\n']);
        let trimmed = line.trim_start_matches([' ', '\t']);
        let leading = line.len() - trimmed.len();
        if leading > 3 {
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if hashes == 0 || hashes > 6 {
            continue;
        }
        let rest = &trimmed[hashes..];
        if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
            continue;
        }
        let content_raw = rest.trim();
        // Strip an optional closing hash run preceded by whitespace.
        let content_raw = match content_raw.rfind(|c: char| c != '#') {
            Some(end)
                if end + 1 < content_raw.len() && content_raw[..=end].ends_with([' ', '\t']) =>
            {
                content_raw[..=end].trim_end()
            }
            _ => content_raw,
        };
        if content_raw.is_empty() {
            continue;
        }
        let text = plain_heading_text(content_raw);
        if text.is_empty() {
            continue;
        }
        let anchor_base = normalize_heading_anchor(&text);
        let mut occurrence = occurrences.get(&anchor_base).copied().unwrap_or(0) + 1;
        let mut anchor = if occurrence == 1 {
            anchor_base.clone()
        } else {
            format!("{anchor_base}-{occurrence}")
        };
        while used.contains(&anchor) {
            occurrence += 1;
            anchor = format!("{anchor_base}-{occurrence}");
        }
        occurrences.insert(anchor_base, occurrence);
        used.insert(anchor.clone());
        let content_start = info.range.start
            + (line.len() - trimmed.len())
            + hashes
            + (rest.len() - rest.trim_start().len());
        headings.push(HeadingFact {
            level: hashes as u8,
            normalized: normalize_title(&text),
            anchor,
            range: content_start..content_start + content_raw.len(),
            text,
        });
    }
    headings
}
