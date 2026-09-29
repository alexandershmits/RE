use crate::state::{xp, AppState};
use eframe::egui::{self, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let cur = app.curriculum.clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("📋 Rubric: чек перед публикацией write-up");
        ui.label(RichText::new(format!("{0}/{0} пунктов = отчёт уровня сеньора. Сверяйте КАЖДЫЙ отчёт (+{1} XP за пункт, один раз).", cur.rubric.len(), xp::RUBRIC_ITEM)).weak());
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| {
            for (i, item) in cur.rubric.iter().enumerate() {
                let mut done = app.progress.rubric_done.contains(&i);
                if ui.checkbox(&mut done, format!("{}. {item}", i + 1)).changed() {
                    app.toggle_rubric(i);
                }
            }
            let n = cur.rubric.len();
            let done = (0..n).filter(|i| app.progress.rubric_done.contains(i)).count();
            ui.add_space(8.0);
            ui.add(egui::ProgressBar::new(done as f32 / n.max(1) as f32).text(format!("{done}/{n}")));
            if ui.button("Сбросить").clicked() {
                app.reset_rubric();
            }
        });
    });
}
