use eframe::egui::{self, RichText, ScrollArea};

use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("📓 Журнал");
        ui.label(
            RichText::new(
                "Каждый разобранный бинарь: скриншоты, псевдокод, что понял / что не понял. \
             Write-up каждого PSet — по правилам честности курса.",
            )
            .weak(),
        );
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .small_button("📤 Экспорт журнала в ~/re50-journal.md")
                .clicked()
            {
                match app.export_journal() {
                    Ok(p) => app.toast(format!("Журнал сохранён: {p}"), ctx),
                    Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
                }
            }
        });
        let mut text = app.progress.journal.clone();
        let response = ScrollArea::vertical().id_salt("journal").show(ui, |ui| {
            ui.add_sized(
                [ui.available_width(), 500.0],
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(24)
                    .code_editor(),
            )
        });
        if response.inner.changed() {
            app.progress.journal = text.clone();
        }
        // save button below also persists
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Символов: {}", app.progress.journal.len()))
                    .weak()
                    .size(11.0),
            );
            if ui.button("💾 Сохранить").clicked() {
                app.progress.journal = text;
                app.save();
                app.toast("Журнал сохранён", ctx);
            }
        });
    });
}
