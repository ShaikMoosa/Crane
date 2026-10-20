use crate::{
    extract_tags, parse_preview, validate_note, Inline, MarkdownBlock, Note, Store, ValidationCode,
    ValidationError,
};
use std::collections::HashSet;

pub const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILENAME_CHARS: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedBytes {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportEntry {
    pub relative_path: String,
    pub contents: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportManifest {
    pub entries: Vec<ExportEntry>,
}

pub fn parse_markdown(
    name: &str,
    bytes: &[u8],
    timestamp: u64,
    id: String,
) -> Result<Note, ValidationError> {
    ensure_markdown_extension(name)?;
    let source = std::str::from_utf8(bytes).map_err(|_| {
        ValidationError::new(
            ValidationCode::InvalidUtf8,
            format!("{name} is not valid UTF-8 Markdown."),
        )
    })?;
    let (heading, body) = split_title_heading(source);
    let note = Note {
        id,
        title: heading.unwrap_or_else(|| filename_title(name)),
        body: body.to_string(),
        tags: extract_tags(body),
        created_at: timestamp,
        updated_at: timestamp,
        pinned: false,
        archived: false,
        trashed: false,
        extensions: Default::default(),
    };
    validate_note(&note).map_err(|error| {
        ValidationError::new(error.code, format!("Could not import {name}: {error}"))
    })?;
    Ok(note)
}

pub fn serialize_markdown(note: &Note) -> String {
    let title = if note.title.trim().is_empty() {
        "Untitled note"
    } else {
        note.title.as_str()
    };
    format!("# {title}\n\n{}", note.body)
}

pub fn suggested_markdown_filename(note: &Note) -> String {
    format!("{}.md", sanitize_filename_stem(&note.title))
}

pub fn prepare_import(
    files: Vec<NamedBytes>,
    timestamp: u64,
    mut ids: impl Iterator<Item = String>,
) -> Result<Vec<Note>, ValidationError> {
    let total = files.iter().try_fold(0usize, |sum, file| {
        sum.checked_add(file.bytes.len()).ok_or_else(|| {
            ValidationError::new(
                ValidationCode::BatchTooLarge,
                "The selected Markdown batch exceeds 64 MB.",
            )
        })
    })?;
    if total > MAX_BATCH_BYTES {
        return Err(ValidationError::new(
            ValidationCode::BatchTooLarge,
            "The selected Markdown batch exceeds the 64 MB limit.",
        ));
    }

    files
        .into_iter()
        .map(|file| {
            let id = ids.next().ok_or_else(|| {
                ValidationError::new(
                    ValidationCode::MissingImportId,
                    "Crane could not assign an ID to every imported note.",
                )
            })?;
            parse_markdown(&file.name, &file.bytes, timestamp, id)
        })
        .collect()
}

pub fn build_export_manifest(store: &Store) -> Result<ExportManifest, ValidationError> {
    let mut used_notes = HashSet::new();
    let mut used_archive = HashSet::new();
    let mut entries = Vec::new();
    for note in store.notes.iter().filter(|note| !note.trashed) {
        validate_note(note)?;
        let (folder, used) = if note.archived {
            ("Archive", &mut used_archive)
        } else {
            ("Notes", &mut used_notes)
        };
        let stem = unique_filename_stem(&note.title, used);
        entries.push(ExportEntry {
            relative_path: format!("{folder}/{stem}.md"),
            contents: serialize_markdown(note).into_bytes(),
        });
    }
    Ok(ExportManifest { entries })
}

pub fn excerpt_from(markdown: &str, max_chars: usize) -> String {
    let document = parse_preview(markdown);
    let mut parts = Vec::new();
    for block in document.blocks {
        match block {
            MarkdownBlock::Paragraph(inlines)
            | MarkdownBlock::Heading {
                content: inlines, ..
            }
            | MarkdownBlock::Quote(inlines) => collect_excerpt_text(&inlines, &mut parts),
            MarkdownBlock::List { items, .. } => {
                for item in items {
                    collect_excerpt_text(&item.content, &mut parts);
                }
            }
            MarkdownBlock::Code { .. } | MarkdownBlock::Rule => {}
        }
    }

    let text = parts.join(" ");
    let words: Vec<_> = text
        .split_whitespace()
        .filter(|word| !is_tag_token(word))
        .collect();
    let compact = words.join(" ");
    if compact.is_empty() {
        return "No additional text".into();
    }
    if compact.chars().count() <= max_chars {
        return compact;
    }
    if max_chars == 0 {
        return String::new();
    }
    let mut shortened: String = compact.chars().take(max_chars.saturating_sub(1)).collect();
    shortened = shortened.trim_end().to_string();
    shortened.push('…');
    shortened
}

fn collect_excerpt_text(inlines: &[Inline], output: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Text(value) => output.push(value.clone()),
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Link {
                label: children, ..
            } => {
                collect_excerpt_text(children, output);
            }
            Inline::Code(_) | Inline::ImageAlt(_) => {}
        }
    }
}

fn is_tag_token(word: &str) -> bool {
    let candidate = word.trim_matches(|character: char| {
        !character.is_alphanumeric() && !matches!(character, '#' | '_' | '-' | '/')
    });
    !candidate.is_empty() && extract_tags(candidate).len() == 1 && candidate.starts_with('#')
}

fn ensure_markdown_extension(name: &str) -> Result<(), ValidationError> {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let is_markdown = basename
        .rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("md"));
    if is_markdown {
        Ok(())
    } else {
        Err(ValidationError::new(
            ValidationCode::UnsupportedExtension,
            format!("{name} is not a Markdown (.md) file."),
        ))
    }
}

fn filename_title(name: &str) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    basename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("Untitled note")
        .to_string()
}

fn split_title_heading(source: &str) -> (Option<String>, &str) {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim().is_empty() {
            offset += line.len();
            continue;
        }
        let Some(title) = content.strip_prefix("# ") else {
            return (None, source);
        };
        if title.trim().is_empty() {
            return (None, source);
        }
        let after_heading = offset + line.len();
        let mut body = &source[after_heading..];
        if let Some(end) = body.find('\n') {
            if body[..=end].trim().is_empty() {
                body = &body[end + 1..];
            }
        } else if body.trim().is_empty() {
            body = "";
        }
        return (Some(title.trim().to_string()), body);
    }
    (None, source)
}

fn unique_filename_stem(title: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_filename_stem(title);
    let mut candidate = base.clone();
    let mut suffix = 2usize;
    while !used.insert(candidate.to_uppercase()) {
        let suffix_text = format!(" {suffix}");
        let available = MAX_FILENAME_CHARS.saturating_sub(suffix_text.chars().count());
        let prefix: String = base.chars().take(available).collect();
        candidate = format!("{prefix}{suffix_text}");
        suffix += 1;
    }
    candidate
}

fn sanitize_filename_stem(title: &str) -> String {
    let filtered: String = title
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
        .collect();
    let trimmed = filtered.trim().trim_end_matches([' ', '.']);
    let mut stem: String = trimmed.chars().take(MAX_FILENAME_CHARS).collect();
    if stem.is_empty() || is_windows_reserved_name(&stem) {
        stem = "Untitled note".into();
    }
    stem
}

fn is_windows_reserved_name(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or(stem).to_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || base
            .strip_prefix("COM")
            .or_else(|| base.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(
                    number,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
}
