use crate::state::AppState;
use eframe::egui::{self, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("📓 Журнал");
        ui.label(RichText::new("Каждый разобранный бинарь: скриншоты, псевдокод, что понял / что не понял. Write-up каждого PSet — по правилам честности курса. Сохраняется автоматически.").weak());
        ui.separator();
        if ui.small_button("📤 Экспорт журнала в re50-journal.md").clicked() {
            match app.export_journal() {
                Ok(p) => app.toast(format!("Журнал сохранён: {p}"), &ctx),
                Err(e) => app.toast(format!("Ошибка: {e}"), &ctx),
            }
        }
        let mut text = app.progress.journal.clone();
        let changed = ScrollArea::vertical()
            .id_salt("journal")
            .show(ui, |ui| {
                let size = [ui.available_width(), (ui.available_height() - 30.0).max(200.0)];
                ui.add_sized(size, egui::TextEdit::multiline(&mut text).code_editor()).changed()
            })
            .inner;
        if changed {
            app.progress.journal = text;
            app.mark_dirty();
        }
        ui.label(RichText::new(format!("Символов: {}", app.progress.journal.chars().count())).weak().size(11.0));
    });
}
