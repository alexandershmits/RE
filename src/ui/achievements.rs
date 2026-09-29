use eframe::egui::{self, RichText, ScrollArea};

use super::WARN;
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("🏅 Ачивки");
            ui.label(RichText::new(format!(
                "Получено {} из {} · {} XP",
                app.progress.achievements.len(),
                app.curriculum.achievements.len(),
                app.progress.xp
            )));
            ui.separator();
            egui::Grid::new("achgrid")
                .num_columns(3)
                .spacing([16.0, 12.0])
                .show(ui, |ui| {
                    for a in &app.curriculum.achievements {
                        let got = app.progress.achievements.contains(&a.id);
                        ui.vertical(|ui| {
                            let name = if got {
                                RichText::new(&a.name).size(18.0).strong()
                            } else {
                                RichText::new("🔒 ? ? ?").weak().size(18.0)
                            };
                            ui.label(name);
                            if got {
                                ui.label(RichText::new(&a.desc).size(11.5));
                                ui.label(
                                    RichText::new(format!("+{} XP", a.xp))
                                        .color(WARN)
                                        .size(11.0),
                                );
                            } else {
                                ui.label(
                                    RichText::new("Условие скрыто — иди учись :)")
                                        .weak()
                                        .size(11.0),
                                );
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    });
}
