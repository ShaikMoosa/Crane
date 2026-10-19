use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_TITLE_BYTES: usize = 500;
pub const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    pub schema_version: u32,
    pub notes: Vec<Note>,
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
                },
            ],
        }
    }
}

pub fn validate_note(note: &Note) -> Result<(), String> {
    if note.id.trim().is_empty() {
        return Err("A note id is required.".into());
    }
    if note.title.len() > MAX_TITLE_BYTES {
        return Err("The note title is too long.".into());
    }
    if note.body.len() > MAX_BODY_BYTES {
        return Err("The note is larger than the 2 MB limit.".into());
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
        let raw = word[1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '/');
        if raw.is_empty() || raw.split('/').any(|segment| segment.is_empty()) {
            continue;
        }
        if raw.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/')) {
            let tag = raw.to_lowercase();
            if seen.insert(tag.clone()) {
                tags.push(tag);
            }
        }
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_unique_nested_tags() {
        assert_eq!(extract_tags("#Design and #work/Notes, then #design"), vec!["design", "work/notes"]);
    }

    #[test]
    fn validates_note_size_limits() {
        let mut note = Store::seed(100).notes.remove(0);
        note.title = "x".repeat(MAX_TITLE_BYTES + 1);
        assert!(validate_note(&note).is_err());
    }
}

