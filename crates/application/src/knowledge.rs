use crane_domain::knowledge::{
    build_tag_tree, normalize_title, tag_in_subtree, HeadingResolution, KnowledgeIndex, NoteFacts,
    NoteStatistics, Resolution, TagNode, WikiLink,
};
use crane_domain::{excerpt_from, Note, Store};
use std::collections::HashMap;
use std::ops::Range;

pub const QUICK_OPEN_LIMIT: usize = 50;
pub const COMPLETION_LIMIT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionView {
    Notes,
    Pinned,
    Today,
    Todos,
    Untagged,
    Archive,
    Trash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    QuickOpen {
        query: String,
        include_archived: bool,
    },
    CompleteWikiLink {
        fragment: String,
    },
    ResolveWikiLink {
        link: WikiLink,
    },
    KnowledgeFor {
        note_id: String,
    },
    TagTree {
        include_archived: bool,
    },
    NotesForCollection {
        collection: CollectionView,
        tag_path: Option<String>,
        query: String,
    },
    CollectionCounts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryData {
    QuickOpen(Vec<QuickOpenHit>),
    Completion(Vec<CompletionCandidate>),
    Resolution(ResolutionView),
    Knowledge(Box<KnowledgeView>),
    TagTree(Vec<TagNode>),
    NoteIds(Vec<String>),
    CollectionCounts(SidebarCounts),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResponse {
    pub store_revision: u64,
    pub data: QueryData,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuickOpenHit {
    pub note_id: String,
    pub title: String,
    pub excerpt: String,
    pub updated_at: u64,
    pub pinned: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompletionCandidate {
    Title {
        note_id: String,
        title: String,
    },
    Heading {
        note_id: String,
        heading_text: String,
        anchor: String,
    },
}

impl CompletionCandidate {
    pub fn insert_text(&self) -> &str {
        match self {
            CompletionCandidate::Title { title, .. } => title,
            CompletionCandidate::Heading { heading_text, .. } => heading_text,
        }
    }
    pub fn note_id(&self) -> &str {
        match self {
            CompletionCandidate::Title { note_id, .. }
            | CompletionCandidate::Heading { note_id, .. } => note_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolutionView {
    Resolved {
        note_id: String,
        heading: HeadingResolution,
    },
    Unresolved {
        target_title: String,
    },
    Ambiguous {
        candidates: Vec<AmbiguousCandidate>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmbiguousCandidate {
    pub note_id: String,
    pub title: String,
    pub excerpt: String,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnowledgeView {
    pub statistics: NoteStatistics,
    pub outline: Vec<OutlineItem>,
    pub outgoing: Vec<OutgoingLinkView>,
    pub backlinks: Vec<BacklinkView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    pub level: u8,
    pub text: String,
    pub anchor: String,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutgoingLinkView {
    pub link: WikiLink,
    pub state: LinkState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkState {
    Resolved {
        note_id: String,
        heading_found: bool,
    },
    Unresolved,
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BacklinkView {
    pub source_note_id: String,
    pub source_title: String,
    pub display_text: String,
    pub context_excerpt: String,
    pub range: Range<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SidebarCounts {
    pub notes: usize,
    pub pinned: usize,
    pub today: usize,
    pub todos: usize,
    pub untagged: usize,
    pub archive: usize,
    pub trash: usize,
}

fn active(facts_vis: Option<&crane_domain::knowledge::NoteVisibility>) -> bool {
    facts_vis.is_some_and(|v| !v.archived && !v.trashed)
}

fn in_scope(index: &KnowledgeIndex, note_id: &str, include_archived: bool) -> bool {
    match index.visibility(note_id) {
        Some(v) => !v.trashed && (include_archived || !v.archived),
        None => false,
    }
}

fn store_order(store: &Store) -> HashMap<String, usize> {
    store
        .notes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect()
}

fn note_by_id<'a>(store: &'a Store, id: &str) -> Option<&'a Note> {
    store.notes.iter().find(|n| n.id == id)
}

pub fn answer_query(
    store: &Store,
    index: &KnowledgeIndex,
    day_bounds: (u64, u64),
    revision: u64,
    query: Query,
) -> QueryResponse {
    let data = match query {
        Query::QuickOpen {
            query,
            include_archived,
        } => QueryData::QuickOpen(quick_open(store, index, &query, include_archived)),
        Query::CompleteWikiLink { fragment } => {
            QueryData::Completion(complete(store, index, &fragment))
        }
        Query::ResolveWikiLink { link } => QueryData::Resolution(resolve_view(store, index, &link)),
        Query::KnowledgeFor { note_id } => {
            QueryData::Knowledge(Box::new(knowledge_for(store, index, &note_id)))
        }
        Query::TagTree { include_archived } => {
            QueryData::TagTree(tag_tree(store, index, include_archived))
        }
        Query::NotesForCollection {
            collection,
            tag_path,
            query,
        } => QueryData::NoteIds(notes_for_collection(
            store,
            index,
            day_bounds,
            collection,
            tag_path.as_deref(),
            &query,
        )),
        Query::CollectionCounts => {
            QueryData::CollectionCounts(collection_counts(store, index, day_bounds))
        }
    };
    QueryResponse {
        store_revision: revision,
        data,
    }
}

fn quick_open(
    store: &Store,
    index: &KnowledgeIndex,
    query: &str,
    include_archived: bool,
) -> Vec<QuickOpenHit> {
    let mut scored: Vec<(u8, QuickOpenHit)> = Vec::new();
    let normalized_query = normalize_title(query);
    let raw_query = query.trim().to_lowercase();
    for note in &store.notes {
        if !in_scope(index, &note.id, include_archived) {
            continue;
        }
        let Some(facts) = index.facts(&note.id) else {
            continue;
        };
        let vis = index.visibility(&note.id).unwrap();
        let hit = QuickOpenHit {
            note_id: note.id.clone(),
            title: note.title.clone(),
            excerpt: excerpt_from(&note.body, 120),
            updated_at: note.updated_at,
            pinned: vis.pinned,
            archived: vis.archived,
        };
        if normalized_query.is_empty() {
            scored.push((0, hit));
            continue;
        }
        let rank = if facts.normalized_title == normalized_query {
            0
        } else if facts.normalized_title.starts_with(&normalized_query) {
            1
        } else if facts.normalized_title.contains(&normalized_query) {
            2
        } else if facts
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&raw_query))
        {
            3
        } else if facts.search_body.contains(&raw_query) {
            4
        } else {
            continue;
        };
        scored.push((rank, hit));
    }
    scored.sort_by(|(rank_a, a), (rank_b, b)| {
        rank_a
            .cmp(rank_b)
            .then_with(|| b.pinned.cmp(&a.pinned))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
    scored
        .into_iter()
        .take(QUICK_OPEN_LIMIT)
        .map(|(_, hit)| hit)
        .collect()
}

fn complete(store: &Store, index: &KnowledgeIndex, fragment: &str) -> Vec<CompletionCandidate> {
    if fragment.contains('|') {
        return Vec::new();
    }
    let _ = store;
    if let Some(hash_at) = find_unescaped(fragment, '#') {
        let title_part = &fragment[..hash_at];
        let heading_part = &fragment[hash_at + 1..];
        let probe = WikiLink {
            range: 0..0,
            target_title: title_part.trim().to_string(),
            normalized_title: normalize_title(title_part),
            heading: None,
            alias: None,
            display_text: title_part.trim().to_string(),
        };
        let Resolution::Resolved { note_id, .. } = index.resolve(&probe) else {
            return Vec::new();
        };
        if !active(index.visibility(&note_id)) {
            return Vec::new();
        }
        let Some(facts) = index.facts(&note_id) else {
            return Vec::new();
        };
        let needle = normalize_title(heading_part);
        return facts
            .headings
            .iter()
            .filter(|h| needle.is_empty() || h.normalized.contains(&needle))
            .take(COMPLETION_LIMIT)
            .map(|h| CompletionCandidate::Heading {
                note_id: note_id.clone(),
                heading_text: h.text.clone(),
                anchor: h.anchor.clone(),
            })
            .collect();
    }
    let needle = normalize_title(fragment);
    let mut candidates: Vec<(bool, &NoteFacts)> = Vec::new();
    for note in &store.notes {
        if !active(index.visibility(&note.id)) {
            continue;
        }
        let Some(facts) = index.facts(&note.id) else {
            continue;
        };
        if facts.normalized_title.is_empty() {
            continue;
        }
        if needle.is_empty() || facts.normalized_title.contains(&needle) {
            candidates.push((facts.normalized_title.starts_with(&needle), facts));
        }
    }
    candidates.sort_by(|(prefix_a, a), (prefix_b, b)| {
        prefix_b
            .cmp(prefix_a)
            .then_with(|| {
                let va = index
                    .visibility(&a.note_id)
                    .map(|v| v.updated_at)
                    .unwrap_or(0);
                let vb = index
                    .visibility(&b.note_id)
                    .map(|v| v.updated_at)
                    .unwrap_or(0);
                vb.cmp(&va)
            })
            .then_with(|| a.note_id.cmp(&b.note_id))
    });
    candidates
        .into_iter()
        .take(COMPLETION_LIMIT)
        .map(|(_, facts)| CompletionCandidate::Title {
            note_id: facts.note_id.clone(),
            title: facts.title.clone(),
        })
        .collect()
}

fn find_unescaped(text: &str, needle: char) -> Option<usize> {
    let mut escaped = false;
    for (i, ch) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == needle {
            return Some(i);
        }
    }
    None
}

fn resolve_view(store: &Store, index: &KnowledgeIndex, link: &WikiLink) -> ResolutionView {
    match index.resolve(link) {
        Resolution::Resolved { note_id, heading } => ResolutionView::Resolved { note_id, heading },
        Resolution::Unresolved { .. } => ResolutionView::Unresolved {
            target_title: link.target_title.clone(),
        },
        Resolution::Ambiguous { candidate_ids } => {
            let order = store_order(store);
            let mut candidates: Vec<AmbiguousCandidate> = candidate_ids
                .iter()
                .filter_map(|id| {
                    note_by_id(store, id).map(|note| AmbiguousCandidate {
                        note_id: note.id.clone(),
                        title: note.title.clone(),
                        excerpt: excerpt_from(&note.body, 120),
                        updated_at: note.updated_at,
                    })
                })
                .collect();
            candidates.sort_by_key(|c| order.get(&c.note_id).copied().unwrap_or(usize::MAX));
            ResolutionView::Ambiguous { candidates }
        }
    }
}

fn knowledge_for(store: &Store, index: &KnowledgeIndex, note_id: &str) -> KnowledgeView {
    let Some(facts) = index.facts(note_id) else {
        return KnowledgeView::default();
    };
    let _ = store;
    let outgoing = facts
        .links
        .iter()
        .map(|link| {
            let state = match index.resolve(link) {
                Resolution::Resolved {
                    note_id: target,
                    heading,
                } => LinkState::Resolved {
                    note_id: target,
                    heading_found: !matches!(heading, HeadingResolution::Missing),
                },
                Resolution::Unresolved { .. } => LinkState::Unresolved,
                Resolution::Ambiguous { .. } => LinkState::Ambiguous,
            };
            OutgoingLinkView {
                link: link.clone(),
                state,
            }
        })
        .collect();
    let mut backlinks: Vec<BacklinkView> = index
        .backlinks(note_id)
        .into_iter()
        .map(|link| BacklinkView {
            source_note_id: link.source_note_id,
            source_title: link.source_title,
            display_text: link.display_text,
            context_excerpt: link.context_excerpt,
            range: link.range,
        })
        .collect();
    backlinks.sort_by(|a, b| {
        let ua = index
            .visibility(&a.source_note_id)
            .map(|v| v.updated_at)
            .unwrap_or(0);
        let ub = index
            .visibility(&b.source_note_id)
            .map(|v| v.updated_at)
            .unwrap_or(0);
        ub.cmp(&ua)
            .then_with(|| a.source_note_id.cmp(&b.source_note_id))
    });
    KnowledgeView {
        statistics: facts.statistics,
        outline: facts
            .headings
            .iter()
            .map(|h| OutlineItem {
                level: h.level,
                text: h.text.clone(),
                anchor: h.anchor.clone(),
                range: h.range.clone(),
            })
            .collect(),
        outgoing,
        backlinks,
    }
}

fn tag_tree(store: &Store, index: &KnowledgeIndex, include_archived: bool) -> Vec<TagNode> {
    let sets: Vec<Vec<String>> = store
        .notes
        .iter()
        .filter(|note| in_scope(index, &note.id, include_archived))
        .filter_map(|note| index.facts(&note.id).map(|f| f.tags.clone()))
        .collect();
    build_tag_tree(&sets)
}

fn collection_counts(
    store: &Store,
    index: &KnowledgeIndex,
    day_bounds: (u64, u64),
) -> SidebarCounts {
    let mut counts = SidebarCounts::default();
    for collection in [
        CollectionView::Notes,
        CollectionView::Pinned,
        CollectionView::Today,
        CollectionView::Todos,
        CollectionView::Untagged,
        CollectionView::Archive,
        CollectionView::Trash,
    ] {
        let n = notes_for_collection(store, index, day_bounds, collection, None, "").len();
        match collection {
            CollectionView::Notes => counts.notes = n,
            CollectionView::Pinned => counts.pinned = n,
            CollectionView::Today => counts.today = n,
            CollectionView::Todos => counts.todos = n,
            CollectionView::Untagged => counts.untagged = n,
            CollectionView::Archive => counts.archive = n,
            CollectionView::Trash => counts.trash = n,
        }
    }
    counts
}

pub fn notes_for_collection(
    store: &Store,
    index: &KnowledgeIndex,
    day_bounds: (u64, u64),
    collection: CollectionView,
    tag_path: Option<&str>,
    query: &str,
) -> Vec<String> {
    let needle = query.trim().to_lowercase();
    let mut notes: Vec<&Note> = store
        .notes
        .iter()
        .filter(|note| {
            let Some(vis) = index.visibility(&note.id) else {
                return false;
            };
            let is_active = !vis.archived && !vis.trashed;
            let in_collection = match collection {
                CollectionView::Notes => is_active,
                CollectionView::Pinned => is_active && vis.pinned,
                CollectionView::Today => {
                    is_active && note.updated_at >= day_bounds.0 && note.updated_at < day_bounds.1
                }
                CollectionView::Todos => {
                    is_active
                        && index
                            .facts(&note.id)
                            .is_some_and(|f| f.statistics.tasks > f.statistics.completed_tasks)
                }
                CollectionView::Untagged => {
                    is_active && index.facts(&note.id).is_some_and(|f| f.tags.is_empty())
                }
                CollectionView::Archive => vis.archived && !vis.trashed,
                CollectionView::Trash => vis.trashed,
            };
            if !in_collection {
                return false;
            }
            if let Some(prefix) = tag_path {
                let Some(facts) = index.facts(&note.id) else {
                    return false;
                };
                if !facts.tags.iter().any(|t| tag_in_subtree(t, prefix)) {
                    return false;
                }
            }
            if !needle.is_empty() {
                let in_title = note.title.to_lowercase().contains(&needle);
                let in_body = note.body.to_lowercase().contains(&needle);
                let in_tags = note.tags.iter().any(|t| t.to_lowercase().contains(&needle));
                if !in_title && !in_body && !in_tags {
                    return false;
                }
            }
            true
        })
        .collect();
    notes.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    notes.into_iter().map(|n| n.id.clone()).collect()
}
