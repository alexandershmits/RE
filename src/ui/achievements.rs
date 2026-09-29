use super::theme::{good, warn};
use crate::state::AppState;
use eframe::egui::{self, RichText, ScrollArea};

const CARD_WIDTH: f32 = 300.0;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let cur = app.curriculum.clone();
    egui::CentralPanel::default().show(ui, |ui| {
        let earned: Vec<_> = cur
            .achievements
            .iter()
            .filter(|a| app.progress.achievements.contains(&a.id))
            .collect();
        let total_xp: u32 = cur.achievements.iter().map(|a| a.xp).sum();
        ui.heading("🏅 Ачивки");
        ui.label(format!(
            "Получено {} из {} · {} XP из {total_xp} за ачивки",
            earned.len(),
            cur.achievements.len(),
            earned.iter().map(|a| a.xp).sum::<u32>()
        ));
        ui.add(
            egui::ProgressBar::new(earned.len() as f32 / cur.achievements.len().max(1) as f32)
                .desired_height(8.0),
        );
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| {
            let columns = ((ui.available_width() / CARD_WIDTH).floor() as usize).clamp(1, 4);
            let width = ui.available_width() / columns as f32 - 16.0;
            egui::Grid::new("achgrid")
                .num_columns(columns)
                .spacing([16.0, 14.0])
                .min_col_width(width)
                .show(ui, |ui| {
                    for (i, a) in cur.achievements.iter().enumerate() {
                        ui.vertical(|ui| {
                            ui.set_max_width(width);
                            if app.progress.achievements.contains(&a.id) {
                                ui.label(RichText::new(&a.name).size(17.0).strong());
                                ui.label(RichText::new(&a.desc).size(12.0));
                                ui.label(
                                    RichText::new(format!("+{} XP · получено ✔", a.xp))
                                        .color(good())
                                        .size(11.0),
                                );
                            } else {
                                ui.label(RichText::new("🔒 ? ? ?").weak().size(17.0));
                                ui.label(
                                    RichText::new("Условие скрыто — иди учись :)")
                                        .weak()
                                        .size(11.0),
                                );
                                ui.label(
                                    RichText::new(format!("+{} XP", a.xp))
                                        .color(warn())
                                        .size(11.0),
                                );
                            }
                        });
                        if (i + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
    });
}
