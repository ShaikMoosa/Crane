pub mod parser;

pub use parser::{
    normalize_heading_anchor, normalize_title, parse_headings, parse_wiki_links, HeadingFact,
    HeadingTarget, WikiLink,
};
