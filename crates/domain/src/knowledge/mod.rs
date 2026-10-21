pub mod index;
pub mod parser;
pub mod statistics;
pub mod tags;

pub use index::{
    context_excerpt, HeadingResolution, InboundLink, KnowledgeIndex, NoteVisibility, Resolution,
};
pub use parser::{
    normalize_heading_anchor, normalize_title, parse_headings, parse_wiki_links, HeadingFact,
    HeadingTarget, WikiLink,
};
pub use statistics::{note_statistics, NoteStatistics};
pub use tags::{build_tag_tree, tag_in_subtree, TagNode};

use crate::{extract_tags, Note};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteFacts {
    pub note_id: String,
    pub title: String,
    pub normalized_title: String,
    pub headings: Vec<HeadingFact>,
    pub links: Vec<WikiLink>,
    pub tags: Vec<String>,
    pub statistics: NoteStatistics,
    pub search_body: String,
}

pub fn parse_note_facts(note: &Note) -> NoteFacts {
    NoteFacts {
        note_id: note.id.clone(),
        title: note.title.clone(),
        normalized_title: normalize_title(&note.title),
        headings: parse_headings(&note.body),
        links: parse_wiki_links(&note.body),
        tags: note.tags.clone(),
        statistics: note_statistics(&note.body),
        search_body: note.body.to_lowercase(),
    }
}

pub fn parse_draft_facts(note_id: &str, title: &str, body: &str) -> NoteFacts {
    NoteFacts {
        note_id: note_id.to_string(),
        title: title.to_string(),
        normalized_title: normalize_title(title),
        headings: parse_headings(body),
        links: parse_wiki_links(body),
        tags: extract_tags(body),
        statistics: note_statistics(body),
        search_body: body.to_lowercase(),
    }
}
