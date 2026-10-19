use crate::{ValidationCode, ValidationError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_TITLE_BYTES: usize = 500;
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_RENDERABLE_TIMESTAMP: u64 = 8_640_000_000_000_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub pinned: bool,
    pub archived: bool,
    pub trashed: bool,
    #[serde(default, flatten)]
    pub extensions: Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    pub schema_version: u32,
    pub notes: Vec<Note>,
    #[serde(default, flatten)]
    pub extensions: Map<String, Value>,
}

impl Store {
    pub fn seed(timestamp: u64) -> Self {
        let day = 86_400_000;
        Self {
            schema_version: SCHEMA_VERSION,
            notes: vec![
                Note {
                    id: "welcome-to-crane".into(),
                    title: "Welcome to Crane".into(),
                    body: "# A quiet place for bright ideas\n\nCrane keeps your notes on this computer. Write in Markdown, use #tags to connect ideas, and let the interface stay out of your way.\n\n- [x] Create a first note\n- [ ] Shape it into something useful\n- [ ] Keep the spark".into(),
                    tags: vec!["tags".into()],
                    created_at: timestamp.saturating_sub(day * 2),
                    updated_at: timestamp.saturating_sub(3_600_000),
                    pinned: true,
                    archived: false,
                    trashed: false,
                    extensions: Map::new(),
                },
                Note {
                    id: "weekend-reading".into(),
                    title: "Weekend reading".into(),
                    body: "## On the desk\n\n- The Creative Act\n- A Pattern Language\n- Ways of Seeing\n\nNotes belong under #reading/list.".into(),
                    tags: vec!["reading/list".into()],
                    created_at: timestamp.saturating_sub(day * 3),
                    updated_at: timestamp.saturating_sub(day),
                    pinned: false,
                    archived: false,
                    trashed: false,
                    extensions: Map::new(),
                },
                Note {
                    id: "product-ritual".into(),
                    title: "A small product ritual".into(),
                    body: "Before adding a feature, write down the behavior in plain language. Then make the smallest test fail. #work/process\n\n> Clarity is a form of speed.".into(),
                    tags: vec!["work/process".into()],
                    created_at: timestamp.saturating_sub(day * 5),
                    updated_at: timestamp.saturating_sub(day * 2),
                    pinned: false,
                    archived: false,
                    trashed: false,
                    extensions: Map::new(),
                },
            ],
            extensions: Map::new(),
        }
    }
}

pub fn validate_note(note: &Note) -> Result<(), ValidationError> {
    if note.id.trim().is_empty() {
        return Err(ValidationError::new(
            ValidationCode::MissingId,
            "A note id is required.",
        ));
    }
    if note.title.len() > MAX_TITLE_BYTES {
        return Err(ValidationError::new(
            ValidationCode::TitleTooLong,
            "The note title is too long.",
        ));
    }
    if note.body.len() > MAX_BODY_BYTES {
        return Err(ValidationError::new(
            ValidationCode::BodyTooLarge,
            "The note is larger than the 2 MB limit.",
        ));
    }
    if note.created_at > MAX_RENDERABLE_TIMESTAMP || note.updated_at > MAX_RENDERABLE_TIMESTAMP {
        return Err(ValidationError::new(
            ValidationCode::TimestampOutOfRange,
            "A note timestamp is outside the supported date range.",
        ));
    }
    Ok(())
}

pub fn validate_store(store: &Store) -> Result<(), ValidationError> {
    if store.schema_version != SCHEMA_VERSION {
        return Err(ValidationError::new(
            ValidationCode::UnsupportedSchema,
            format!("Unsupported store version {}.", store.schema_version),
        ));
    }

    let mut ids = HashSet::new();
    for note in &store.notes {
        validate_note(note)?;
        if !ids.insert(note.id.as_str()) {
            return Err(ValidationError::new(
                ValidationCode::DuplicateId,
                format!("The store contains duplicate note id {}.", note.id),
            ));
        }
    }
    Ok(())
}

pub fn extract_tags(source: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = HashSet::new();
    for word in source.split_whitespace() {
        if !word.starts_with('#') || word.len() == 1 {
            continue;
        }
        let raw = word[1..]
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '/');
        if raw.is_empty() || raw.split('/').any(|segment| segment.is_empty()) {
            continue;
        }
        if raw
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
        {
            let tag = raw.to_lowercase();
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
        }
    }
    tags
}
