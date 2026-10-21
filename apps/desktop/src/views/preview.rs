use crane_domain::knowledge::{normalize_heading_anchor, WikiLink};
use crane_domain::{Inline, MarkdownBlock, MarkdownDocument};
use eframe::egui::{self, RichText};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PreviewAction {
    #[default]
    None,
    HttpError(String),
    WikiLink(WikiLink),
}

pub fn show(
    ui: &mut egui::Ui,
    document: &MarkdownDocument,
    scroll_to_anchor: Option<&str>,
) -> PreviewAction {
    show_with_opener(
        ui,
        document,
        &mut crane_platform::open_http_link,
        scroll_to_anchor,
    )
}

#[doc(hidden)]
pub fn show_with_opener(
    ui: &mut egui::Ui,
    document: &MarkdownDocument,
    opener: &mut impl FnMut(&str) -> Result<(), crane_platform::PlatformError>,
    scroll_to_anchor: Option<&str>,
) -> PreviewAction {
    let mut action = PreviewAction::None;
    egui::ScrollArea::vertical()
        .id_salt("crane-markdown-preview")
        .show(ui, |ui| {
            for block in &document.blocks {
                match block {
                    MarkdownBlock::Heading { level, content } => {
                        let size = (30.0 - f32::from(*level) * 2.0).max(18.0);
                        let heading = ui.horizontal_wrapped(|ui| {
                            render_inlines(
                                ui,
                                content,
                                InlineStyle {
                                    strong: true,
                                    size: Some(size),
                                    ..InlineStyle::default()
                                },
                                opener,
                                &mut action,
                            );
                        });
                        if scroll_to_anchor.is_some_and(|target| {
                            normalize_heading_anchor(&inline_text(content)) == target
                        }) {
                            ui.scroll_to_rect(heading.response.rect, Some(egui::Align::Min));
                        }
                    }
                    MarkdownBlock::Paragraph(content) => {
                        ui.horizontal_wrapped(|ui| {
                            render_inlines(
                                ui,
                                content,
                                InlineStyle::default(),
                                opener,
                                &mut action,
                            );
                        });
                    }
                    MarkdownBlock::Quote(content) => {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("❯").italics());
                            render_inlines(
                                ui,
                                content,
                                InlineStyle {
                                    italics: true,
                                    ..InlineStyle::default()
                                },
                                opener,
                                &mut action,
                            );
                        });
                    }
                    MarkdownBlock::Code { language, text } => {
                        if let Some(language) = language {
                            ui.small(language);
                        }
                        ui.monospace(text);
                    }
                    MarkdownBlock::List { ordered, items } => {
                        for (index, item) in items.iter().enumerate() {
                            let marker = match item.checked {
                                Some(true) => "☑".to_owned(),
                                Some(false) => "☐".to_owned(),
                                None if *ordered => format!("{}.", index + 1),
                                None => "•".to_owned(),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.label(marker);
                                render_inlines(
                                    ui,
                                    &item.content,
                                    InlineStyle::default(),
                                    opener,
                                    &mut action,
                                );
                            });
                        }
                    }
                    MarkdownBlock::Rule => {
                        ui.separator();
                    }
                }
                ui.add_space(7.0);
            }
        });
    action
}

#[derive(Clone, Copy, Default)]
struct InlineStyle {
    strong: bool,
    italics: bool,
    size: Option<f32>,
}

fn render_inlines(
    ui: &mut egui::Ui,
    inlines: &[Inline],
    style: InlineStyle,
    opener: &mut impl FnMut(&str) -> Result<(), crane_platform::PlatformError>,
    action: &mut PreviewAction,
) {
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::ImageAlt(text) => {
                ui.label(styled_text(text, style));
            }
            Inline::WikiLink(link) => {
                let name = format!("Open linked note: {}", link.display_text);
                if ui
                    .link(styled_text(&link.display_text, style))
                    .accessible_name(&name)
                    .clicked()
                {
                    *action = PreviewAction::WikiLink(link.clone());
                }
            }
            Inline::Code(text) => {
                ui.monospace(text);
            }
            Inline::Emphasis(children) => {
                render_inlines(
                    ui,
                    children,
                    InlineStyle {
                        italics: true,
                        ..style
                    },
                    opener,
                    action,
                );
            }
            Inline::Strong(children) => {
                render_inlines(
                    ui,
                    children,
                    InlineStyle {
                        strong: true,
                        ..style
                    },
                    opener,
                    action,
                );
            }
            Inline::Link { label, url } => {
                if ui.link(styled_text(inline_text(label), style)).clicked() {
                    if let Err(error) = opener(url) {
                        *action = PreviewAction::HttpError(error.to_string());
                    }
                }
            }
        }
    }
}

fn styled_text(text: impl Into<String>, style: InlineStyle) -> RichText {
    let mut text = RichText::new(text);
    if style.strong {
        text = text.strong();
    }
    if style.italics {
        text = text.italics();
    }
    if let Some(size) = style.size {
        text = text.size(size);
    }
    text
}

fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(value) | Inline::Code(value) | Inline::ImageAlt(value) => {
                text.push_str(value)
            }
            Inline::Emphasis(children) | Inline::Strong(children) => {
                text.push_str(&inline_text(children));
            }
            Inline::Link { label, .. } => text.push_str(&inline_text(label)),
            Inline::WikiLink(link) => text.push_str(&link.display_text),
        }
    }
    text
}
