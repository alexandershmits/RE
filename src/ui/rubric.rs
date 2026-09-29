use eframe::egui::{self, RichText, ScrollArea};

use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("📋 Rubric: чек перед публикацией write-up");
        ui.label(
            RichText::new("10/10 пунктов = отчёт уровня сеньора. Сверяйте КАЖДЫЙ отчёт.").weak(),
        );
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| {
            let rubric_items = app.curriculum.rubric.clone();
            for (i, item) in rubric_items.iter().enumerate() {
                let key = format!("rubric:{}", i);
                let mut done = app.progress.lab_steps_done.contains(&key);
                if ui
                    .checkbox(&mut done, format!("{}. {item}", i + 1))
                    .changed()
                {
                    app.toggle_lab_step(&key);
                    app.save();
                }
            }
            let n = app.curriculum.rubric.len();
            let done = (0..n)
                .filter(|i| app.progress.lab_steps_done.contains(&format!("rubric:{i}")))
                .count();
            ui.add_space(8.0);
            ui.add(
                egui::ProgressBar::new(done as f32 / n.max(1) as f32).text(format!("{done}/{n}")),
            );
            if ui.button("Сбросить").clicked() {
                for i in 0..n {
                    app.progress.lab_steps_done.remove(&format!("rubric:{i}"));
                }
                app.save();
            }
        });
    });
}
