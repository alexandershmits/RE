use eframe::egui::{self, RichText, ScrollArea};

use super::theme::accent;
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("🔗 Библиотека ресурсов");
            ui.label(
                RichText::new("Все инструменты и материалы курса — бесплатные или с free-версией.")
                    .weak(),
            );
            ui.separator();
            let mut last_cat = String::new();
            for r in &app.curriculum.resources {
                if r.category != last_cat {
                    ui.add_space(6.0);
                    ui.strong(RichText::new(&r.category).color(accent()));
                    last_cat = r.category.clone();
                }
                ui.horizontal(|ui| {
                    if ui.link(&r.name).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(&r.url));
                    }
                    ui.label(RichText::new(&r.url).weak().size(10.5));
                });
            }
        });
    });
}
