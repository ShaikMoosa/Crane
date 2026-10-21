use pulldown_cmark::{Event, Options, Parser, Tag};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoteStatistics {
    pub characters: usize,
    pub words: usize,
    pub paragraphs: usize,
    pub headings: usize,
    pub tasks: usize,
    pub completed_tasks: usize,
    pub reading_minutes: u64,
}

pub fn note_statistics(body: &str) -> NoteStatistics {
    let words = body.split_whitespace().count();
    let mut paragraphs = 0usize;
    let mut headings = 0usize;
    let mut tasks = 0usize;
    let mut completed_tasks = 0usize;
    for event in Parser::new_ext(body, Options::ENABLE_TASKLISTS) {
        match event {
            Event::Start(Tag::Paragraph) => paragraphs += 1,
            Event::Start(Tag::Heading { .. }) => headings += 1,
            Event::TaskListMarker(checked) => {
                tasks += 1;
                if checked {
                    completed_tasks += 1;
                }
            }
            _ => {}
        }
    }
    NoteStatistics {
        characters: body.chars().count(),
        words,
        paragraphs,
        headings,
        tasks,
        completed_tasks,
        reading_minutes: if words == 0 {
            0
        } else {
            ((words as u64) + 199) / 200
        },
    }
}
