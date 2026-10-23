use crate::theme;
use crane_domain::{highlight_markdown, MarkdownRole, MarkdownSpan};
use crane_storage::ThemeChoice;
use eframe::egui::{
    text::{LayoutJob, TextFormat},
    FontFamily, FontId, Stroke,
};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

const BODY_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 24.0;

#[derive(Clone, Debug)]
struct CacheKey {
    note_id: String,
    revision: u64,
    content_len: usize,
    content_hash: u64,
}

#[derive(Default)]
pub struct MarkdownHighlightCache {
    key: Option<CacheKey>,
    spans: Vec<MarkdownSpan>,
    parse_count: u64,
}

impl MarkdownHighlightCache {
    fn spans<'a>(&'a mut self, note_id: &str, revision: u64, source: &str) -> &'a [MarkdownSpan] {
        let content_hash = fingerprint(source);
        let reusable = self.key.as_ref().is_some_and(|key| {
            key.note_id == note_id
                && key.content_len == source.len()
                && key.content_hash == content_hash
        });
        if !reusable {
            self.spans = highlight_markdown(source);
            self.parse_count = self.parse_count.saturating_add(1);
            self.key = Some(CacheKey {
                note_id: note_id.to_owned(),
                revision,
                content_len: source.len(),
                content_hash,
            });
        } else if let Some(key) = &mut self.key {
            key.revision = revision;
        }
        &self.spans
    }
}

#[derive(Default)]
pub struct MarkdownLayouter {
    cache: MarkdownHighlightCache,
    last_error: Option<MarkdownLayoutError>,
}

impl MarkdownLayouter {
    pub fn layout_job(
        &mut self,
        note_id: &str,
        revision: u64,
        source: &str,
        wrap_width: f32,
        choice: ThemeChoice,
    ) -> Result<LayoutJob, MarkdownLayoutError> {
        let spans = self.cache.spans(note_id, revision, source);
        match layout_job_from_spans(source, spans, wrap_width, choice) {
            Ok(job) => {
                self.last_error = None;
                Ok(job)
            }
            Err(error) => {
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn parse_count(&self) -> u64 {
        self.cache.parse_count
    }

    pub fn last_error(&self) -> Option<&MarkdownLayoutError> {
        self.last_error.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownLayoutError;

impl fmt::Display for MarkdownLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Markdown styling unavailable; editing is unchanged.")
    }
}

impl std::error::Error for MarkdownLayoutError {}

#[doc(hidden)]
pub fn layout_job_from_spans(
    source: &str,
    spans: &[MarkdownSpan],
    wrap_width: f32,
    choice: ThemeChoice,
) -> Result<LayoutJob, MarkdownLayoutError> {
    validate_spans(source, spans)?;
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.keep_trailing_whitespace = true;
    for span in spans {
        job.append(
            &source[span.byte_range.clone()],
            0.0,
            format_for(&span.role, choice),
        );
    }
    Ok(job)
}

pub fn plain_text_job(source: &str, wrap_width: f32, choice: ThemeChoice) -> LayoutJob {
    let mut job = LayoutJob::simple(
        source.to_owned(),
        interface_font(BODY_SIZE),
        theme::tokens(choice).text_primary,
        wrap_width,
    );
    job.keep_trailing_whitespace = true;
    for section in &mut job.sections {
        section.format.line_height = Some(LINE_HEIGHT);
    }
    job
}

fn validate_spans(source: &str, spans: &[MarkdownSpan]) -> Result<(), MarkdownLayoutError> {
    if source.is_empty() {
        return spans.is_empty().then_some(()).ok_or(MarkdownLayoutError);
    }
    if spans.is_empty()
        || spans.first().is_none_or(|span| span.byte_range.start != 0)
        || spans
            .last()
            .is_none_or(|span| span.byte_range.end != source.len())
    {
        return Err(MarkdownLayoutError);
    }
    let mut expected = 0usize;
    for span in spans {
        if span.byte_range.start != expected
            || span.byte_range.start >= span.byte_range.end
            || span.byte_range.end > source.len()
            || !source.is_char_boundary(span.byte_range.start)
            || !source.is_char_boundary(span.byte_range.end)
        {
            return Err(MarkdownLayoutError);
        }
        expected = span.byte_range.end;
    }
    (expected == source.len())
        .then_some(())
        .ok_or(MarkdownLayoutError)
}

fn format_for(role: &MarkdownRole, choice: ThemeChoice) -> TextFormat {
    let palette = theme::tokens(choice);
    let mut format = TextFormat {
        font_id: interface_font(BODY_SIZE),
        line_height: Some(LINE_HEIGHT),
        color: palette.text_primary,
        ..TextFormat::default()
    };
    match role {
        MarkdownRole::Plain => {}
        MarkdownRole::Marker => format.color = palette.markdown_marker,
        MarkdownRole::Heading(level) => {
            format.color = palette.markdown_heading;
            format.font_id = interface_font(heading_size(*level));
        }
        MarkdownRole::Strong => format.color = palette.markdown_emphasis,
        MarkdownRole::Emphasis => {
            format.color = palette.markdown_emphasis;
            format.italics = true;
        }
        MarkdownRole::Strikethrough => {
            format.color = palette.markdown_emphasis;
            format.strikethrough = Stroke::new(1.0, palette.markdown_emphasis);
        }
        MarkdownRole::LinkText | MarkdownRole::LinkDestination | MarkdownRole::WikiLink => {
            format.color = palette.markdown_link;
        }
        MarkdownRole::Tag => format.color = palette.markdown_tag,
        MarkdownRole::QuoteMarker => format.color = palette.markdown_quote,
        MarkdownRole::ListMarker | MarkdownRole::TaskMarker { .. } => {
            format.color = palette.markdown_marker;
        }
        MarkdownRole::InlineCode | MarkdownRole::CodeBlock => {
            format.font_id = FontId::new(14.0, FontFamily::Monospace);
            format.color = palette.markdown_code;
            format.background = palette.markdown_code_background;
        }
    }
    format
}

fn interface_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Crane Interface".into()))
}

fn heading_size(level: u8) -> f32 {
    match level.clamp(1, 6) {
        1 => 22.0,
        2 => 21.0,
        3 => 20.0,
        4 => 19.0,
        _ => 18.0,
    }
}

fn fingerprint(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}
