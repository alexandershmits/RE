use super::theme::accent;
use crate::state::AppState;
use eframe::egui::{self, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let cur = app.curriculum.clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("🔗 Библиотека ресурсов");
            ui.label(
                RichText::new("Все инструменты и материалы курса — бесплатные или с free-версией.")
                    .weak(),
            );
            ui.separator();
            let mut categories: Vec<&str> = Vec::new();
            for r in &cur.resources {
                if !categories.contains(&r.category.as_str()) {
                    categories.push(&r.category);
                }
            }
            for category in categories {
                ui.add_space(6.0);
                ui.strong(RichText::new(category).color(accent()));
                for r in cur.resources.iter().filter(|r| r.category == category) {
                    ui.horizontal_wrapped(|ui| {
                        if ui.link(&r.name).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(&r.url));
                        }
                        ui.label(RichText::new(&r.url).weak().size(10.5));
                    });
                }
            }
        });
    });
}
