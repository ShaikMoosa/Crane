use crane_application::knowledge::{AmbiguousCandidate, CompletionCandidate, Query};
use crane_domain::knowledge::{is_markdown_code_at, HeadingResolution};
use std::ops::Range;

/// An unclosed `[[wiki-link` fragment under the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiFragment {
    /// Byte range of `[[` plus the fragment text, up to the caret.
    pub replace_range: Range<usize>,
    /// Raw text between `[[` and the caret.
    pub raw: String,
}

/// Find the wiki-link fragment under `caret_byte`, if any.
///
/// Returns `None` when the caret is not after an unescaped, unclosed `[[`
/// on the caret line, during alias editing (`|`), or inside fenced or inline
/// code. IME composition is tracked separately by the caller.
pub fn wiki_fragment_at(body: &str, caret_byte: usize) -> Option<WikiFragment> {
    if caret_byte > body.len() || !body.is_char_boundary(caret_byte) {
        return None;
    }
    let before = &body[..caret_byte];
    let open = before.rfind("[[")?;
    let after_open = &before[open + 2..];
    if after_open.contains("]]") || after_open.contains('|') || after_open.contains('\n') {
        return None;
    }
    let backslashes = before[..open]
        .chars()
        .rev()
        .take_while(|c| *c == '\\')
        .count();
    if backslashes % 2 == 1 {
        return None;
    }
    if is_markdown_code_at(before, open) {
        return None;
    }
    Some(WikiFragment {
        replace_range: open..caret_byte,
        raw: after_open.to_owned(),
    })
}

/// Rebuild the full `[[...]]` link text for a completion candidate.
pub fn completion_replacement(raw: &str, candidate: &CompletionCandidate) -> String {
    match candidate {
        CompletionCandidate::Title { .. } => format!("[[{}]]", candidate.insert_text()),
        CompletionCandidate::Heading { .. } => {
            let title = raw.split('#').next().unwrap_or(raw);
            format!("[[{title}#{}]]", candidate.insert_text())
        }
    }
}

/// Convert a char index into a byte index, clamped to the string end.
pub fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .nth(char_index)
        .unwrap_or(text.len())
}

/// Convert a byte index into a char index, rounding down to a boundary.
pub fn byte_to_char(text: &str, byte_index: usize) -> usize {
    let clamped = byte_index.min(text.len());
    let mut boundary = clamped;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary].chars().count()
}

/// Editor-side wiki-link completion state.
#[derive(Clone, Debug, Default)]
pub struct CompletionState {
    pub fragment: Option<WikiFragment>,
    pub candidates: Vec<CompletionCandidate>,
    pub active: usize,
    pub sequence: u64,
    pub ime_composing: bool,
    dismissed: Option<String>,
}

impl CompletionState {
    pub fn popup_visible(&self) -> bool {
        self.fragment.is_some() && !self.candidates.is_empty() && !self.ime_composing
    }

    pub fn clear(&mut self) {
        self.fragment = None;
        self.candidates.clear();
        self.active = 0;
        self.dismissed = None;
    }

    pub fn dismiss(&mut self) {
        self.dismissed = self.fragment.as_ref().map(|fragment| fragment.raw.clone());
        self.fragment = None;
        self.candidates.clear();
        self.active = 0;
    }

    pub fn is_dismissed(&self, raw: &str) -> bool {
        self.dismissed.as_deref() == Some(raw)
    }

    /// Track the caret fragment, emitting a completion query only when the
    /// fragment text changes. A dismissed fragment stays suppressed until the
    /// user edits it into something else.
    pub fn set_fragment(&mut self, fragment: Option<WikiFragment>, emit: &mut dyn FnMut(Query)) {
        let raw = fragment.as_ref().map(|fragment| fragment.raw.clone());
        let changed = raw != self.fragment.as_ref().map(|fragment| fragment.raw.clone());
        if !changed {
            return;
        }
        if self.dismissed.is_some() && self.dismissed != raw {
            self.dismissed = None;
        }
        self.fragment = fragment;
        self.candidates.clear();
        self.active = 0;
        let mut send = None;
        match &self.fragment {
            Some(found) if !self.is_dismissed(&found.raw) => {
                send = Some(found.raw.clone());
            }
            _ => {}
        }
        if let Some(raw) = send {
            self.sequence = self.sequence.saturating_add(1);
            emit(Query::CompleteWikiLink { fragment: raw });
        }
    }

    /// Replace the fragment with the candidate link text. Returns the new
    /// caret char index (just after the closing `]]`).
    pub fn apply_candidate(&mut self, body: &mut String, index: usize) -> Option<usize> {
        let fragment = self.fragment.clone()?;
        let candidate = self.candidates.get(index)?;
        let replacement = completion_replacement(&fragment.raw, candidate);
        body.replace_range(fragment.replace_range.clone(), &replacement);
        let caret_byte = fragment.replace_range.start + replacement.len();
        let caret_char = byte_to_char(body, caret_byte);
        self.clear();
        Some(caret_char)
    }
}

/// Navigation requested from a wiki link or the chooser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnowledgeNavIntent {
    OpenNote {
        note_id: String,
        heading: Option<HeadingResolution>,
    },
    CreateNote {
        title: String,
    },
}

/// Ambiguity/unresolved chooser state.
#[derive(Clone, Debug, Default)]
pub struct ChooserState {
    pub open: bool,
    pub title: String,
    /// Empty candidates means the link is unresolved: offer create only.
    pub candidates: Vec<AmbiguousCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChooserIntent {
    OpenNote(String),
    CreateNote(String),
    Close,
}

pub fn open_chooser(state: &mut ChooserState, title: String, candidates: Vec<AmbiguousCandidate>) {
    state.open = true;
    state.title = title;
    state.candidates = candidates;
}

pub fn close_chooser(state: &mut ChooserState) {
    state.open = false;
    state.candidates.clear();
}

pub fn show_chooser(
    ctx: &eframe::egui::Context,
    state: &mut ChooserState,
) -> Option<ChooserIntent> {
    if !state.open {
        return None;
    }
    let mut intent = None;
    let mut close_requested = false;
    eframe::egui::Window::new("wiki-link-chooser")
        .title_bar(true)
        .collapsible(false)
        .resizable(false)
        .anchor(eframe::egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if ui.input_mut(|input| {
                input.consume_key(eframe::egui::Modifiers::NONE, eframe::egui::Key::Escape)
            }) {
                close_requested = true;
            }
            ui.heading("Choose link target");
            if state.candidates.is_empty() {
                ui.label(format!("No note titled \"{}\".", state.title));
                if ui
                    .button(format!("Create note \"{}\"", state.title))
                    .clicked()
                {
                    intent = Some(ChooserIntent::CreateNote(state.title.clone()));
                    close_requested = true;
                }
            } else {
                ui.label(format!(
                    "{} notes share this title:",
                    state.candidates.len()
                ));
                for candidate in &state.candidates {
                    ui.separator();
                    if ui
                        .button(format!("Choose linked note: {}", candidate.title))
                        .clicked()
                    {
                        intent = Some(ChooserIntent::OpenNote(candidate.note_id.clone()));
                        close_requested = true;
                    }
                    ui.small(&candidate.excerpt);
                    ui.small(format!(
                        "Updated {}",
                        crate::views::restore::format_timestamp_millis(candidate.updated_at)
                    ));
                }
            }
            if ui.button("Cancel").clicked() {
                close_requested = true;
            }
        });
    if close_requested {
        close_chooser(state);
        if intent.is_none() {
            intent = Some(ChooserIntent::Close);
        }
    }
    intent
}

#[cfg(test)]
mod tests {
    use super::*;

    fn title(id: &str, title: &str) -> CompletionCandidate {
        CompletionCandidate::Title {
            note_id: id.to_owned(),
            title: title.to_owned(),
        }
    }

    fn heading(id: &str, text: &str) -> CompletionCandidate {
        CompletionCandidate::Heading {
            note_id: id.to_owned(),
            heading_text: text.to_owned(),
            anchor: text.to_owned(),
        }
    }

    #[test]
    fn fragment_rules_cover_links_code_and_escapes() {
        let fragment = wiki_fragment_at("See [[Fie", 9).expect("fragment");
        assert_eq!(fragment.raw, "Fie");
        assert_eq!(fragment.replace_range, 4..9);
        assert!(wiki_fragment_at("See [[Fie]]", 11).is_none());
        assert!(wiki_fragment_at("See [[A|B", 10).is_none());
        assert!(wiki_fragment_at("See \\[[Fie", 10).is_none());
        assert!(wiki_fragment_at("See \\\\[[Fie", 11).is_some());
        assert!(wiki_fragment_at("```\n[[Fie", 9).is_none());
        assert!(wiki_fragment_at("```\ncode\n```\n[[Fie", 18).is_some());
        assert!(wiki_fragment_at("~~~~\n[[Fie", 10).is_none());
        assert!(wiki_fragment_at("````\n```\n[[Fie", 14).is_none());
        assert!(wiki_fragment_at("`code\n[[Fie", 11).is_none());
        assert!(wiki_fragment_at("a `[[Fie", 8).is_none());
        assert!(wiki_fragment_at("`x` [[Fie", 9).is_some());
        assert!(wiki_fragment_at("[[one\n[[two", 11).is_some());
        assert!(wiki_fragment_at("plain", 5).is_none());
        assert!(wiki_fragment_at("[[Fie", 99).is_none());
        assert!(wiki_fragment_at("é", 1).is_none());
    }

    #[test]
    fn replacement_rebuilds_full_link() {
        assert_eq!(
            completion_replacement("Fie", &title("f", "Field Notes")),
            "[[Field Notes]]"
        );
        assert_eq!(
            completion_replacement("Field Notes#On", &heading("f", "Onboarding")),
            "[[Field Notes#Onboarding]]"
        );
    }

    #[test]
    fn char_byte_conversions_clamp_and_round() {
        assert_eq!(char_to_byte("héllo", 1), 1);
        assert_eq!(char_to_byte("héllo", 2), 3);
        assert_eq!(char_to_byte("héllo", 99), 6);
        assert_eq!(byte_to_char("héllo", 2), 1);
        assert_eq!(byte_to_char("héllo", 99), 5);
    }

    #[test]
    fn completion_visibility_tracks_fragment_candidates_and_ime() {
        let mut state = CompletionState::default();
        assert!(!state.popup_visible());
        state.fragment = wiki_fragment_at("[[Fie", 5);
        assert!(!state.popup_visible());
        state.candidates = vec![title("f", "Field Notes")];
        assert!(state.popup_visible());
        state.ime_composing = true;
        assert!(!state.popup_visible());
        state.ime_composing = false;
        state.dismiss();
        assert!(!state.popup_visible());
        assert!(state.is_dismissed("Fie"));
        assert!(!state.is_dismissed("Fiel"));
        state.clear();
        assert!(!state.is_dismissed("Fie"));
    }

    #[test]
    fn changed_fragment_clears_stale_candidates_before_new_results_arrive() {
        let mut state = CompletionState {
            fragment: wiki_fragment_at("[[Old", 5),
            candidates: vec![title("old", "Old")],
            ..CompletionState::default()
        };
        let mut emitted = Vec::new();

        state.set_fragment(wiki_fragment_at("[[New", 5), &mut |query| {
            emitted.push(query)
        });

        assert!(state.candidates.is_empty());
        assert_eq!(state.active, 0);
        assert_eq!(emitted.len(), 1);
    }
}
