use crate::Store;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Collection {
    Notes,
    Pinned,
    Archive,
    Trash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteFilter {
    pub collection: Collection,
    pub tag: Option<String>,
    pub query: String,
}

pub fn query_note_ids(store: &Store, filter: &NoteFilter) -> Vec<String> {
    let tag = filter.tag.as_deref().map(str::to_lowercase);
    let query = filter.query.trim().to_lowercase();
    let mut notes: Vec<_> = store
        .notes
        .iter()
        .filter(|note| match filter.collection {
            Collection::Notes => !note.archived && !note.trashed,
            Collection::Pinned => note.pinned && !note.archived && !note.trashed,
            Collection::Archive => note.archived && !note.trashed,
            Collection::Trash => note.trashed,
        })
        .filter(|note| {
            tag.as_ref().is_none_or(|selected| {
                note.tags
                    .iter()
                    .any(|candidate| candidate.to_lowercase() == *selected)
            })
        })
        .filter(|note| {
            query.is_empty()
                || note.title.to_lowercase().contains(&query)
                || note.body.to_lowercase().contains(&query)
                || note
                    .tags
                    .iter()
                    .any(|candidate| candidate.to_lowercase().contains(&query))
        })
        .collect();

    notes.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then(Ordering::Equal)
    });
    notes.into_iter().map(|note| note.id.clone()).collect()
}
