use crane_desktop::markdown_layouter::{layout_job_from_spans, plain_text_job, MarkdownLayouter};
use crane_domain::{MarkdownRole, MarkdownSpan};
use crane_storage::ThemeChoice;
use eframe::egui::{Color32, FontFamily};

fn assert_job_coverage(source: &str, job: &eframe::egui::text::LayoutJob) {
    assert_eq!(job.text, source);
    if source.is_empty() {
        assert!(job.sections.is_empty());
        return;
    }
    assert_eq!(job.sections.first().unwrap().byte_range.start.0, 0);
    assert_eq!(job.sections.last().unwrap().byte_range.end.0, source.len());
    for pair in job.sections.windows(2) {
        assert_eq!(pair[0].byte_range.end, pair[1].byte_range.start);
    }
}

fn section_for<'a>(
    source: &str,
    job: &'a eframe::egui::text::LayoutJob,
    needle: &str,
) -> &'a eframe::egui::text::LayoutSection {
    let start = source.find(needle).unwrap();
    let range = start..start + needle.len();
    job.sections
        .iter()
        .find(|section| {
            section.byte_range.start.0 <= range.start && section.byte_range.end.0 >= range.end
        })
        .unwrap()
}

#[test]
fn cache_reparses_only_for_content_or_note_identity_changes() {
    let mut layouter = MarkdownLayouter::default();
    let source = "# Heading\nBody #tag";

    let first = layouter
        .layout_job("alpha", 1, source, 640.0, ThemeChoice::Light)
        .unwrap();
    assert_job_coverage(source, &first);
    assert_eq!(layouter.parse_count(), 1);

    layouter
        .layout_job("alpha", 1, source, 640.0, ThemeChoice::Light)
        .unwrap();
    layouter
        .layout_job("alpha", 1, source, 420.0, ThemeChoice::Light)
        .unwrap();
    layouter
        .layout_job("alpha", 1, source, 420.0, ThemeChoice::Dark)
        .unwrap();
    assert_eq!(layouter.parse_count(), 1);

    layouter
        .layout_job("alpha", 1, "# Heading\nChanged", 420.0, ThemeChoice::Dark)
        .unwrap();
    assert_eq!(layouter.parse_count(), 2);
    layouter
        .layout_job("beta", 1, source, 420.0, ThemeChoice::Dark)
        .unwrap();
    assert_eq!(layouter.parse_count(), 3);
}

#[test]
fn semantic_roles_map_to_theme_fonts_colors_and_backgrounds() {
    let source = "# Heading\n**bold** [link](https://example.com) #tag `code`";
    let mut layouter = MarkdownLayouter::default();
    let job = layouter
        .layout_job("alpha", 7, source, 600.0, ThemeChoice::Light)
        .unwrap();
    assert_job_coverage(source, &job);
    assert!(job.keep_trailing_whitespace);
    assert_eq!(job.wrap.max_width, 600.0);

    let tokens = crane_desktop::theme::tokens(ThemeChoice::Light);
    assert_eq!(
        section_for(source, &job, "#").format.color,
        tokens.markdown_marker
    );
    let heading = section_for(source, &job, "Heading");
    assert_eq!(heading.format.color, tokens.markdown_heading);
    assert!(heading.format.font_id.size > 16.0);
    assert_eq!(
        section_for(source, &job, "link").format.color,
        tokens.markdown_link
    );
    assert_eq!(
        section_for(source, &job, "#tag").format.color,
        tokens.markdown_tag
    );
    let code = section_for(source, &job, "`code`");
    assert_eq!(code.format.font_id.family, FontFamily::Monospace);
    assert_eq!(code.format.color, tokens.markdown_code);
    assert_eq!(code.format.background, tokens.markdown_code_background);
}

#[test]
fn invalid_spans_fall_back_without_echoing_note_content() {
    let source = "private-secret-content";
    let invalid = vec![MarkdownSpan {
        byte_range: 2..source.len(),
        role: MarkdownRole::Plain,
    }];
    let error = layout_job_from_spans(source, &invalid, 500.0, ThemeChoice::Light)
        .expect_err("missing prefix must be rejected");
    assert_eq!(
        error.to_string(),
        "Markdown styling unavailable; editing is unchanged."
    );
    assert!(!error.to_string().contains(source));

    let fallback = plain_text_job(source, 500.0, ThemeChoice::Light);
    assert_job_coverage(source, &fallback);
    assert_eq!(fallback.sections.len(), 1);
    assert_eq!(fallback.sections[0].byte_range.start.0, 0);
    assert_eq!(fallback.sections[0].byte_range.end.0, source.len());
    assert_eq!(fallback.sections[0].format.background, Color32::TRANSPARENT);
}
