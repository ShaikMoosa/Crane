use super::{parse_note_facts, HeadingTarget, NoteFacts, WikiLink};
use crate::{Note, Store};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoteVisibility {
    pub pinned: bool,
    pub archived: bool,
    pub trashed: bool,
    pub updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadingResolution {
    Found { anchor: String, range: Range<usize> },
    Missing,
    NotRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved {
        note_id: String,
        heading: HeadingResolution,
    },
    Unresolved {
        normalized_title: String,
    },
    Ambiguous {
        candidate_ids: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboundLink {
    pub source_note_id: String,
    pub source_title: String,
    pub range: Range<usize>,
    pub heading: Option<HeadingTarget>,
    pub display_text: String,
    pub context_excerpt: String,
}

#[derive(Clone, Debug, Default)]
pub struct KnowledgeIndex {
    facts: HashMap<String, NoteFacts>,
    visibility: HashMap<String, NoteVisibility>,
    by_title: HashMap<String, Vec<String>>,
    inbound: HashMap<String, Vec<InboundLink>>,
}

pub fn context_excerpt(body: &str, range: &Range<usize>) -> String {
    let start_char = body[..range.start.min(body.len())].chars().count();
    let end_char = body[..range.end.min(body.len())].chars().count();
    let chars: Vec<char> = body.chars().collect();
    let from = start_char.saturating_sub(40);
    let to = (end_char + 40).min(chars.len());
    let window: String = chars[from..to].iter().collect();
    window.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl KnowledgeIndex {
    pub fn build(store: &Store) -> Self {
        let mut index = Self::default();
        for note in &store.notes {
            index.insert_note(note);
        }
        index
    }

    pub fn note_count(&self) -> usize {
        self.facts.len()
    }

    pub fn facts(&self, note_id: &str) -> Option<&NoteFacts> {
        self.facts.get(note_id)
    }

    pub fn visibility(&self, note_id: &str) -> Option<&NoteVisibility> {
        self.visibility.get(note_id)
    }

    pub fn replace_note(&mut self, note: &Note) {
        self.remove_contributions(&note.id);
        self.insert_note(note);
    }

    pub fn update_visibility(&mut self, note: &Note) {
        if !self.facts.contains_key(&note.id) {
            self.replace_note(note);
            return;
        }
        self.visibility.insert(note.id.clone(), visibility_of(note));
    }

    pub fn resolve(&self, link: &WikiLink) -> Resolution {
        let Some(candidates) = self.by_title.get(&link.normalized_title) else {
            return Resolution::Unresolved {
                normalized_title: link.normalized_title.clone(),
            };
        };
        match candidates.len() {
            0 => Resolution::Unresolved {
                normalized_title: link.normalized_title.clone(),
            },
            1 => {
                let note_id = candidates[0].clone();
                let heading = match &link.heading {
                    None => HeadingResolution::NotRequested,
                    Some(target) => {
                        let found = self.facts.get(&note_id).and_then(|facts| {
                            facts
                                .headings
                                .iter()
                                .find(|h| {
                                    h.anchor == target.anchor_base
                                        || h.normalized == target.normalized
                                })
                                .map(|h| HeadingResolution::Found {
                                    anchor: h.anchor.clone(),
                                    range: h.range.clone(),
                                })
                        });
                        found.unwrap_or(HeadingResolution::Missing)
                    }
                };
                Resolution::Resolved { note_id, heading }
            }
            _ => Resolution::Ambiguous {
                candidate_ids: candidates.clone(),
            },
        }
    }

    pub fn backlinks(&self, note_id: &str) -> Vec<InboundLink> {
        let Some(facts) = self.facts.get(note_id) else {
            return Vec::new();
        };
        if facts.normalized_title.is_empty() {
            return Vec::new();
        }
        // Only uniquely resolved inbound links count as backlinks.
        match self.by_title.get(&facts.normalized_title) {
            Some(candidates) if candidates.len() == 1 && candidates[0] == note_id => {}
            _ => return Vec::new(),
        }
        self.inbound
            .get(&facts.normalized_title)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|link| link.source_note_id != note_id || true)
            .collect()
    }

    fn insert_note(&mut self, note: &Note) {
        let facts = parse_note_facts(note);
        self.visibility.insert(note.id.clone(), visibility_of(note));
        if !facts.normalized_title.is_empty() {
            self.by_title
                .entry(facts.normalized_title.clone())
                .or_default()
                .push(note.id.clone());
        }
        for link in &facts.links {
            self.inbound
                .entry(link.normalized_title.clone())
                .or_default()
                .push(InboundLink {
                    source_note_id: note.id.clone(),
                    source_title: note.title.clone(),
                    range: link.range.clone(),
                    heading: link.heading.clone(),
                    display_text: link.display_text.clone(),
                    context_excerpt: context_excerpt(&note.body, &link.range),
                });
        }
        self.facts.insert(note.id.clone(), facts);
    }

    fn remove_contributions(&mut self, note_id: &str) {
        let Some(old) = self.facts.remove(note_id) else {
            return;
        };
        if !old.normalized_title.is_empty() {
            if let Some(ids) = self.by_title.get_mut(&old.normalized_title) {
                ids.retain(|id| id != note_id);
            }
        }
        for link in &old.links {
            if let Some(entries) = self.inbound.get_mut(&link.normalized_title) {
                entries.retain(|entry| entry.source_note_id != note_id);
            }
        }
    }
}

fn visibility_of(note: &Note) -> NoteVisibility {
    NoteVisibility {
        pinned: note.pinned,
        archived: note.archived,
        trashed: note.trashed,
        updated_at: note.updated_at,
    }
}
