use crate::icons::IconId;
use eframe::egui;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatAction {
    Heading(u8),
    Task,
    BulletList,
    NumberedList,
    Quote,
    Bold,
    Italic,
    InlineCode,
    CodeBlock,
    InsertLink { url: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOutcome {
    pub body: String,
    /// Byte range of the selection after the edit.
    pub selection: Range<usize>,
    pub applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormattingKind {
    Heading,
    Task,
    BulletList,
    NumberedList,
    Quote,
    Bold,
    Italic,
    InlineCode,
    CodeBlock,
    Link,
}

#[derive(Clone, Debug, Default)]
pub struct FormattingState {
    pub link_open: bool,
    pub link_url: String,
    /// Live body char range from the previous frame. Clicking a bar button
    /// collapses the widget selection on the click frame, so actions fall
    /// back to this when the live range is collapsed.
    pub last_char_range: Option<(usize, usize)>,
    active: Vec<FormattingKind>,
    restore_body_focus: bool,
}

impl FormattingState {
    pub fn sync_active(&mut self, body: &str, char_range: Option<(usize, usize)>) {
        self.active.clear();
        let Some((first, second)) = char_range else {
            return;
        };
        let first = char_to_byte(body, first);
        let second = char_to_byte(body, second);
        let (start, end) = (first.min(second), first.max(second));
        let line_start = body[..start].rfind('\n').map_or(0, |index| index + 1);
        let line_end = body[end..]
            .find('\n')
            .map_or(body.len(), |index| end + index);
        let line = &body[line_start..line_end];
        if strip_atx(line).is_some() {
            self.active.push(FormattingKind::Heading);
        }
        if task_marker_len(line).is_some() {
            self.active.push(FormattingKind::Task);
        } else if line.starts_with("- ") {
            self.active.push(FormattingKind::BulletList);
        }
        if ordered_marker_len(line).is_some() {
            self.active.push(FormattingKind::NumberedList);
        }
        if line.starts_with("> ") {
            self.active.push(FormattingKind::Quote);
        }
        if surrounded_by(body, start, end, "**") {
            self.active.push(FormattingKind::Bold);
        } else if surrounded_by(body, start, end, "*") {
            self.active.push(FormattingKind::Italic);
        }
        if surrounded_by(body, start, end, "`") {
            self.active.push(FormattingKind::InlineCode);
        }
        if body[..line_start].ends_with("```\n") && body[line_end..].starts_with("\n```") {
            self.active.push(FormattingKind::CodeBlock);
        }
        if self.link_open {
            self.active.push(FormattingKind::Link);
        }
    }

    fn is_active(&self, kind: FormattingKind) -> bool {
        self.active.contains(&kind) || (kind == FormattingKind::Link && self.link_open)
    }

    pub fn take_body_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.restore_body_focus)
    }
}

fn char_to_byte(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .nth(char_index)
        .unwrap_or(text.len())
}

fn surrounded_by(body: &str, start: usize, end: usize, marker: &str) -> bool {
    start >= marker.len() && body[..start].ends_with(marker) && body[end..].starts_with(marker)
}

impl FormatAction {
    fn kind(&self) -> FormattingKind {
        match self {
            Self::Heading(_) => FormattingKind::Heading,
            Self::Task => FormattingKind::Task,
            Self::BulletList => FormattingKind::BulletList,
            Self::NumberedList => FormattingKind::NumberedList,
            Self::Quote => FormattingKind::Quote,
            Self::Bold => FormattingKind::Bold,
            Self::Italic => FormattingKind::Italic,
            Self::InlineCode => FormattingKind::InlineCode,
            Self::CodeBlock => FormattingKind::CodeBlock,
            Self::InsertLink { .. } => FormattingKind::Link,
        }
    }
}

fn clamp_boundary(body: &str, index: usize) -> usize {
    let mut clamped = index.min(body.len());
    while clamped > 0 && !body.is_char_boundary(clamped) {
        clamped -= 1;
    }
    clamped
}

fn normalize_selection(body: &str, selection: Range<usize>) -> (usize, usize) {
    let start = clamp_boundary(body, selection.start.min(selection.end));
    let end = clamp_boundary(body, selection.start.max(selection.end));
    (start, end)
}

/// Byte ranges (without newlines) of the lines intersecting `[start, end)`.
/// An empty selection yields the caret line.
fn line_bounds(body: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    if start == end {
        let line_start = body[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = body[start..]
            .find('\n')
            .map(|i| start + i)
            .unwrap_or(body.len());
        return vec![(line_start, line_end)];
    }
    let mut lines = Vec::new();
    let mut line_start = body[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    loop {
        if line_start >= end && !lines.is_empty() {
            break;
        }
        let line_end = body[line_start..]
            .find('\n')
            .map(|i| line_start + i)
            .unwrap_or(body.len());
        lines.push((line_start, line_end));
        if line_end >= end {
            break;
        }
        line_start = line_end + 1;
    }
    lines
}

fn strip_atx(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &line[hashes..];
    if rest.is_empty() {
        return Some((hashes, ""));
    }
    if rest.starts_with([' ', '\t']) {
        return Some((hashes, rest.trim_start_matches([' ', '\t'])));
    }
    None
}

fn task_marker_len(line: &str) -> Option<usize> {
    let mut chars = line.chars();
    if chars.next() != Some('-') || chars.next() != Some(' ') || chars.next() != Some('[') {
        return None;
    }
    let mark = chars.next()?;
    if chars.next() != Some(']') || chars.next() != Some(' ') {
        return None;
    }
    Some(3 + mark.len_utf8() + 2)
}

fn ordered_marker_len(line: &str) -> Option<usize> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    if rest.starts_with(". ") {
        Some(digits + 2)
    } else {
        None
    }
}

fn apply_line_action(
    body: &str,
    start: usize,
    end: usize,
    transform: &mut dyn FnMut(&str, usize) -> String,
) -> FormatOutcome {
    let lines = line_bounds(body, start, end);
    let mut out = String::with_capacity(body.len() + lines.len() * 4);
    let mut cursor = 0;
    let mut total_delta: i64 = 0;
    let mut first_delta: i64 = 0;
    for (index, (line_start, line_end)) in lines.iter().enumerate() {
        out.push_str(&body[cursor..*line_start]);
        let replacement = transform(&body[*line_start..*line_end], index);
        let delta = replacement.len() as i64 - (*line_end - *line_start) as i64;
        if index == 0 {
            first_delta = delta;
        }
        total_delta += delta;
        out.push_str(&replacement);
        cursor = *line_end;
    }
    out.push_str(&body[cursor..]);
    let new_start = (start as i64 + first_delta).max(0) as usize;
    let new_end = (end as i64 + total_delta).max(0) as usize;
    FormatOutcome {
        body: out,
        selection: new_start.min(new_end)..new_start.max(new_end),
        applied: true,
    }
}

fn wrap_or_toggle(body: &str, start: usize, end: usize, marker: &str) -> FormatOutcome {
    let len = marker.len();
    let surrounded =
        body[..start].ends_with(marker) && body[end.min(body.len())..].starts_with(marker);
    if surrounded && (start < end || start >= len) {
        let mut out = String::with_capacity(body.len());
        out.push_str(&body[..start - len]);
        out.push_str(&body[start..end]);
        out.push_str(&body[end + len..]);
        return FormatOutcome {
            body: out,
            selection: start - len..end - len,
            applied: true,
        };
    }
    if start == end {
        let mut out = String::with_capacity(body.len() + 2 * len);
        out.push_str(&body[..start]);
        out.push_str(marker);
        out.push_str(marker);
        out.push_str(&body[start..]);
        return FormatOutcome {
            body: out,
            selection: start + len..start + len,
            applied: true,
        };
    }
    let mut out = String::with_capacity(body.len() + 2 * len);
    out.push_str(&body[..start]);
    out.push_str(marker);
    out.push_str(&body[start..end]);
    out.push_str(marker);
    out.push_str(&body[end..]);
    FormatOutcome {
        body: out,
        selection: start + len..end + len,
        applied: true,
    }
}

fn apply_code_block(body: &str, start: usize, end: usize) -> FormatOutcome {
    let lines = line_bounds(body, start, end);
    let first_start = lines.first().map(|(s, _)| *s).unwrap_or(0);
    let last_end = lines.last().map(|(_, e)| *e).unwrap_or(0);
    let fenced_before = first_start >= 4 && body[..first_start].ends_with("```\n");
    let fenced_after = body[last_end..].starts_with("\n```")
        && body[last_end..]
            .strip_prefix("\n```")
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('\n'));
    if fenced_before && fenced_after {
        let mut out = String::with_capacity(body.len());
        out.push_str(&body[..first_start - 4]);
        out.push_str(&body[first_start..last_end]);
        out.push_str(&body[last_end + 4..]);
        return FormatOutcome {
            body: out,
            selection: first_start - 4..last_end - 4,
            applied: true,
        };
    }
    let mut out = String::with_capacity(body.len() + 8);
    out.push_str(&body[..first_start]);
    out.push_str("```\n");
    out.push_str(&body[first_start..last_end]);
    out.push_str("\n```");
    out.push_str(&body[last_end..]);
    FormatOutcome {
        body: out,
        selection: first_start + 4..last_end + 4,
        applied: true,
    }
}

fn apply_link(body: &str, start: usize, end: usize, url: &str) -> FormatOutcome {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return FormatOutcome {
            body: body.to_owned(),
            selection: start..end,
            applied: false,
        };
    }
    if start == end {
        let mut out = String::with_capacity(body.len() + url.len() + 4);
        out.push_str(&body[..start]);
        out.push_str("[](");
        out.push_str(url);
        out.push(')');
        out.push_str(&body[start..]);
        return FormatOutcome {
            body: out,
            selection: start + 1..start + 1,
            applied: true,
        };
    }
    let mut out = String::with_capacity(body.len() + url.len() + 4);
    out.push_str(&body[..start]);
    out.push('[');
    out.push_str(&body[start..end]);
    out.push_str("](");
    out.push_str(url);
    out.push(')');
    out.push_str(&body[end..]);
    let new_end = start + 1 + (end - start) + 2 + url.len() + 1;
    FormatOutcome {
        body: out,
        selection: start..new_end,
        applied: true,
    }
}

/// Apply a formatting action to `body` for the byte `selection`.
///
/// The selection is clamped into bounds, rounded down to UTF-8 boundaries,
/// and unordered ends are sorted, so this never panics on hostile ranges.
pub fn apply_format(body: &str, selection: Range<usize>, action: &FormatAction) -> FormatOutcome {
    let (start, end) = normalize_selection(body, selection);
    match action {
        FormatAction::Heading(level) => {
            let level = (*level).clamp(1, 6);
            let prefix = format!("{} ", "#".repeat(level as usize));
            apply_line_action(body, start, end, &mut |line, _| match strip_atx(line) {
                Some((found, rest)) if found == level as usize => rest.to_owned(),
                Some((_, rest)) => format!("{prefix}{rest}"),
                None => format!("{prefix}{line}"),
            })
        }
        FormatAction::Task => {
            apply_line_action(
                body,
                start,
                end,
                &mut |line, _| match task_marker_len(line) {
                    Some(len) if line.as_bytes()[3] == b' ' => line[len..].to_owned(),
                    Some(len) => format!("- [ ] {}", &line[len..]),
                    None => format!("- [ ] {line}"),
                },
            )
        }
        FormatAction::BulletList => apply_line_action(body, start, end, &mut |line, _| {
            line.strip_prefix("- ")
                .map(str::to_owned)
                .unwrap_or_else(|| format!("- {line}"))
        }),
        FormatAction::NumberedList => {
            let lines = line_bounds(body, start, end);
            let all_numbered = lines
                .iter()
                .all(|(s, e)| ordered_marker_len(&body[*s..*e]).is_some());
            apply_line_action(body, start, end, &mut |line, index| {
                let base = ordered_marker_len(line)
                    .map(|len| &line[len..])
                    .unwrap_or(line);
                if all_numbered {
                    base.to_owned()
                } else {
                    format!("{}. {base}", index + 1)
                }
            })
        }
        FormatAction::Quote => apply_line_action(body, start, end, &mut |line, _| {
            line.strip_prefix("> ")
                .map(str::to_owned)
                .unwrap_or_else(|| format!("> {line}"))
        }),
        FormatAction::Bold => wrap_or_toggle(body, start, end, "**"),
        FormatAction::Italic => wrap_or_toggle(body, start, end, "*"),
        FormatAction::InlineCode => wrap_or_toggle(body, start, end, "`"),
        FormatAction::CodeBlock => apply_code_block(body, start, end),
        FormatAction::InsertLink { url } => apply_link(body, start, end, url),
    }
}

pub fn show(ui: &mut egui::Ui, state: &mut FormattingState, enabled: bool) -> Option<FormatAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        let picks: [(IconId, &str, FormatAction); 9] = [
            (IconId::Heading, "Heading", FormatAction::Heading(2)),
            (IconId::ListChecks, "Task list", FormatAction::Task),
            (IconId::List, "Bulleted list", FormatAction::BulletList),
            (
                IconId::ListOrdered,
                "Numbered list",
                FormatAction::NumberedList,
            ),
            (IconId::Quote, "Quote", FormatAction::Quote),
            (IconId::Bold, "Bold", FormatAction::Bold),
            (IconId::Italic, "Italic", FormatAction::Italic),
            (IconId::Code, "Inline code", FormatAction::InlineCode),
            (IconId::Code, "Code block", FormatAction::CodeBlock),
        ];
        for (icon, name, candidate) in picks {
            if formatting_button(ui, icon, name, state.is_active(candidate.kind())).clicked()
                && enabled
            {
                action = Some(candidate);
            }
        }
        if formatting_button(
            ui,
            IconId::Link,
            "Insert link",
            state.is_active(FormattingKind::Link),
        )
        .clicked()
            && enabled
        {
            state.link_open = true;
            state.link_url.clear();
            state.restore_body_focus = false;
        }
    });
    if state.link_open {
        let modal_id = egui::Id::unique("format-link-dialog");
        let area = egui::Modal::default_area(modal_id).accessible_name("Insert link");
        let response = egui::Modal::new(modal_id)
            .area(area)
            .frame(crate::theme::overlay_frame(
                crate::theme::choice_for_context(ui.ctx()),
            ))
            .backdrop_color(crate::theme::modal_backdrop(
                crate::theme::choice_for_context(ui.ctx()),
            ))
            .show(ui.ctx(), |ui| {
                ui.label("Link address");
                ui.add(
                    egui::TextEdit::singleline(&mut state.link_url)
                        .hint_text("https://…")
                        .desired_width(280.0),
                )
                .accessible_name("Link URL");
                ui.horizontal(|ui| {
                    if ui
                        .button("Insert")
                        .accessible_name("Insert link address")
                        .clicked()
                    {
                        action = Some(FormatAction::InsertLink {
                            url: state.link_url.clone(),
                        });
                        state.link_open = false;
                        state.restore_body_focus = true;
                    }
                    if ui
                        .button("Cancel")
                        .accessible_name("Cancel link insertion")
                        .clicked()
                    {
                        state.link_open = false;
                        state.restore_body_focus = true;
                    }
                });
            });
        if response.should_close() {
            state.link_open = false;
            state.restore_body_focus = true;
        }
    }
    action
}

pub fn toolbar_height(ui: &egui::Ui, available_width: f32) -> f32 {
    const TOOL_COUNT: usize = 10;

    let edge = crate::icons::target_size(ui);
    let spacing = ui.spacing().item_spacing;
    let tools_per_row = (((available_width + spacing.x) / (edge + spacing.x)).floor() as usize)
        .clamp(1, TOOL_COUNT);
    let rows = TOOL_COUNT.div_ceil(tools_per_row);
    rows as f32 * edge + rows.saturating_sub(1) as f32 * spacing.y
}

fn formatting_button(
    ui: &mut egui::Ui,
    icon: IconId,
    accessible_name: &str,
    selected: bool,
) -> egui::Response {
    let image = crate::icons::icon_for_text_button(ui, icon, ui.visuals().text_color());
    let edge = crate::icons::target_size(ui);
    let response = ui
        .add_sized([edge, edge], egui::Button::image(image).selected(selected))
        .accessible_name(accessible_name)
        .on_hover_text(accessible_name);
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, enabled, selected, accessible_name)
    });
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(body: &str, selection: Range<usize>, action: FormatAction) -> FormatOutcome {
        apply_format(body, selection, &action)
    }

    #[test]
    fn inline_wrap_unwrap_and_caret() {
        let out = apply("hello", 0..5, FormatAction::Bold);
        assert_eq!(out.body, "**hello**");
        assert_eq!(out.selection, 2..7);
        assert!(out.applied);

        let back = apply(&out.body, out.selection, FormatAction::Bold);
        assert_eq!(back.body, "hello");
        assert_eq!(back.selection, 0..5);

        let caret = apply("hello", 2..2, FormatAction::Bold);
        assert_eq!(caret.body, "he****llo");
        assert_eq!(caret.selection, 4..4);

        let inside = apply("****", 2..2, FormatAction::Bold);
        assert_eq!(inside.body, "");
        assert_eq!(inside.selection, 0..0);

        let italic = apply("hey", 0..3, FormatAction::Italic);
        assert_eq!(italic.body, "*hey*");
        let code = apply("x", 0..1, FormatAction::InlineCode);
        assert_eq!(code.body, "`x`");
    }

    #[test]
    fn heading_prefix_replace_and_remove() {
        let out = apply("Title", 0..5, FormatAction::Heading(2));
        assert_eq!(out.body, "## Title");
        assert_eq!(out.selection, 3..8);

        let back = apply(&out.body, out.selection.clone(), FormatAction::Heading(2));
        assert_eq!(back.body, "Title");

        let replaced = apply("### Title", 0..9, FormatAction::Heading(2));
        assert_eq!(replaced.body, "## Title");

        let caret_line = apply("a\nb", 2..2, FormatAction::Heading(1));
        assert_eq!(caret_line.body, "a\n# b");
    }

    #[test]
    fn line_prefixes_apply_per_line_and_toggle() {
        let out = apply("one\ntwo", 0..7, FormatAction::BulletList);
        assert_eq!(out.body, "- one\n- two");

        let back = apply(&out.body, 0..11, FormatAction::BulletList);
        assert_eq!(back.body, "one\ntwo");

        let numbered = apply("a\nb\nc", 0..5, FormatAction::NumberedList);
        assert_eq!(numbered.body, "1. a\n2. b\n3. c");
        let unnumbered = apply(&numbered.body, 0..17, FormatAction::NumberedList);
        assert_eq!(unnumbered.body, "a\nb\nc");

        let renumber = apply("9. a\nb", 0..6, FormatAction::NumberedList);
        assert_eq!(renumber.body, "1. a\n2. b");

        let quote = apply("a", 0..1, FormatAction::Quote);
        assert_eq!(quote.body, "> a");

        let task = apply("do", 0..2, FormatAction::Task);
        assert_eq!(task.body, "- [ ] do");
        let untask = apply(&task.body, 0..8, FormatAction::Task);
        assert_eq!(untask.body, "do");
        let retask = apply("- [x] do", 0..8, FormatAction::Task);
        assert_eq!(retask.body, "- [ ] do");
    }

    #[test]
    fn code_block_fences_and_unfences() {
        let out = apply("a\nb", 0..3, FormatAction::CodeBlock);
        assert_eq!(out.body, "```\na\nb\n```");
        assert_eq!(out.selection, 4..7);

        let back = apply(&out.body, out.selection.clone(), FormatAction::CodeBlock);
        assert_eq!(back.body, "a\nb");

        let caret = apply("x", 1..1, FormatAction::CodeBlock);
        assert_eq!(caret.body, "```\nx\n```");
    }

    #[test]
    fn insert_link_accepts_http_only() {
        let out = apply(
            "tap",
            0..3,
            FormatAction::InsertLink {
                url: "https://example.com".to_owned(),
            },
        );
        assert_eq!(out.body, "[tap](https://example.com)");
        assert!(out.applied);

        let caret = apply(
            "tap",
            1..1,
            FormatAction::InsertLink {
                url: "http://example.com".to_owned(),
            },
        );
        assert_eq!(caret.body, "t[](http://example.com)ap");
        assert_eq!(caret.selection, 2..2);

        for url in ["file:///etc/passwd", "javascript:alert(1)", "notes/other"] {
            let rejected = apply(
                "tap",
                0..3,
                FormatAction::InsertLink {
                    url: url.to_owned(),
                },
            );
            assert!(!rejected.applied);
            assert_eq!(rejected.body, "tap");
        }
    }

    #[test]
    fn hostile_selections_clamp_without_panic() {
        let body = "héllo wörld";
        let start_inside = apply(body, 0..2, FormatAction::Bold);
        assert_eq!(start_inside.body, "**h**éllo wörld");
        assert_eq!(start_inside.selection, 2..3);

        let end_inside = apply(body, 2..3, FormatAction::Bold);
        assert_eq!(end_inside.body, "h**é**llo wörld");
        assert!(end_inside.applied);

        let swapped = apply("abc", Range { start: 3, end: 0 }, FormatAction::Italic);
        assert_eq!(swapped.body, "*abc*");

        let oob = apply("abc", 0..99, FormatAction::Bold);
        assert_eq!(oob.body, "**abc**");

        let empty = apply("", 0..0, FormatAction::BulletList);
        assert_eq!(empty.body, "- ");
    }
}
