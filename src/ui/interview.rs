use eframe::egui::{self, RichText, ScrollArea};

use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🎤 Банк вопросов интервью RE");
        ui.label(RichText::new("Self-interview: ответьте вслух, потом откройте эталон. Слабые вопросы — в карточки.").weak());
        ui.separator();
        ScrollArea::vertical().id_salt("interview").show(ui, |ui| {
            for q in app.curriculum.interview_questions.iter() {
                ui.collapsing(RichText::new(format!("{} · {}", q.cat, q.q)).strong(), |ui| {
                    ui.label(&q.a);
                });
            }
        });
    });
}
