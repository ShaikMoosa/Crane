use crate::transfers::RestoreDialogModel;
use eframe::egui;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RestoreAction {
    #[default]
    None,
    Confirm,
    Cancel,
}

pub fn show(
    context: &egui::Context,
    model: RestoreDialogModel<'_>,
    request_cancel_focus: bool,
) -> RestoreAction {
    let modal_id = egui::Id::unique("crane-restore-confirmation");
    let area = egui::Modal::default_area(modal_id).accessible_name("Restore Crane backup");
    let choice = crate::theme::choice_for_context(context);
    let response = egui::Modal::new(modal_id)
        .area(area)
        .frame(crate::theme::overlay_frame(choice))
        .backdrop_color(crate::theme::modal_backdrop(choice))
        .show(context, |ui| {
            ui.heading("Restore Crane backup");
            ui.label(format!(
                "Backup date: {}",
                format_timestamp_millis(model.inspection.created_at)
            ));
            ui.label(format!(
                "{} total · {} active · {} archived · {} trash",
                model.inspection.total,
                model.inspection.active,
                model.inspection.archived,
                model.inspection.trashed
            ));
            ui.separator();
            ui.label("This replaces the current Crane library after a safety backup is written.");
            if model.restoring {
                ui.label("Writing safety backup and replacing the library…");
            }
            ui.horizontal(|ui| {
                let cancel = ui
                    .add_enabled(!model.restoring, egui::Button::new("Cancel"))
                    .accessible_name("Cancel restore");
                if request_cancel_focus && !model.restoring {
                    cancel.request_focus();
                }
                let replace = ui
                    .add_enabled(!model.restoring, egui::Button::new("Replace library"))
                    .accessible_name("Replace library");
                (cancel.clicked(), replace.clicked())
            })
            .inner
        });

    if response.inner.1 {
        RestoreAction::Confirm
    } else if response.inner.0 || (!model.restoring && response.should_close()) {
        RestoreAction::Cancel
    } else {
        RestoreAction::None
    }
}

pub(crate) fn format_timestamp_millis(timestamp_millis: u64) -> String {
    let seconds = (timestamp_millis / 1_000).min(i64::MAX as u64) as i64;
    let days = seconds / 86_400;
    let seconds_in_day = seconds % 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}
