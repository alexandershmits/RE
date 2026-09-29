use eframe::egui::{self, RichText, ScrollArea};

use super::ACCENT;
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🗺 Визуальные схемы");
        // Topic graph at top
        if !app.curriculum.topic_map.is_empty() {
            ui.collapsing(RichText::new("🌍 КАРТА КУРСА — как связаны все темы").strong().color(ACCENT).size(15.0), |ui| {
                let mut code = app.curriculum.topic_map.clone();
                egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut code)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(42)
                            .desired_width(760.0)
                            .interactive(false),
                    );
                });
            });
            ui.add_space(6.0);
        }
        ui.label(RichText::new("Моноширинные схемы-шпаргалки: стек, vtable, PE, GOT/PLT, hollowing, пайплайн. Вернитесь к ним, когда тема встретится в практике.").weak());
        ui.separator();
        ScrollArea::vertical().id_salt("diagrams").show(ui, |ui| {
            let ds = app.curriculum.diagrams.clone();
            for d in ds {
                ui.collapsing(RichText::new(format!("📐 {} [{}]", d.title, d.tag)).strong(), |ui| {
                    let mut code = d.code.clone();
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(12, 13, 18))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut code)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(d.code.lines().count() + 1)
                                    .desired_width(720.0)
                                    .interactive(false),
                            );
                        });
                });
            }
        });
    });
}
