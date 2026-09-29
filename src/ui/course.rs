use eframe::egui::{self, RichText, ScrollArea};

use super::{ACCENT, GOOD, WARN};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = &ui.ctx().clone();
    egui::Panel::right("week_list")
        .exact_size(240.0)
        .show(ui, |ui| {
            ui.heading("Недели");
            ui.separator();
            ScrollArea::vertical().id_salt("weeklist").show(ui, |ui| {
                for (i, w) in app.curriculum.weeks.iter().enumerate() {
                    let done = app.progress.weeks_done.contains(&w.id);
                    let mark = if done { "✅ " } else { "▫️ " };
                    let label = format!("{}{} · {}", mark, w.label(), w.title);
                    if ui
                        .selectable_label(app.selected_week == i, RichText::new(label).size(12.5))
                        .clicked()
                    {
                        app.selected_week = i;
                    }
                }
            });
        });

    egui::CentralPanel::default().show(ui, |ui| {
        let Some(w) = app.curriculum.weeks.get(app.selected_week) else {
            return;
        };
        let week = w.clone();
        ScrollArea::vertical().id_salt("weekdetail").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new(week.label()).color(ACCENT));
                ui.strong(&week.title);
            });
            ui.label(RichText::new(app.curriculum.module_name(week.module)).weak());
            ui.horizontal(|ui| {
                if ui
                    .small_button("📂 Открыть лабу недели (→ ~/re50-lab/)")
                    .clicked()
                {
                    match app.export_week_lab(&week.id) {
                        Some(dir) => app.toast(format!("TASK.md создан: {dir}"), ctx),
                        None => app.toast("Не удалось создать лабу", ctx),
                    }
                }
            });
            ui.separator();

            // 🎯 problem-first case
            if let Some(case) = &week.case {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(38, 30, 40))
                    .stroke(egui::Stroke::new(1.0_f32, WARN))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.strong(
                            RichText::new("🎯 Проблема недели — сначала ЗАЧЕМ, потом КАК:")
                                .color(WARN),
                        );
                        ui.label(case);
                    });
                ui.add_space(8.0);
            }

            // 📖 lectures
            if !week.lectures.is_empty() {
                ui.heading("📖 Лекции");
                for l in &week.lectures {
                    ui.label(format!("• {l}"));
                }
                ui.add_space(8.0);
            }

            // 🧪 lab
            if let Some(lab) = &week.lab {
                ui.heading(format!("🧪 {}", lab.title));
                for (i, step) in lab.steps.iter().enumerate() {
                    let key = format!("{}:{}", week.id, i);
                    let mut checked = app.progress.lab_steps_done.contains(&key);
                    if ui.checkbox(&mut checked, step).changed() {
                        app.toggle_lab_step(&key);
                        app.save();
                    }
                }
                ui.add_space(8.0);
            }

            // 📝 psets
            if !week.psets.is_empty() {
                ui.heading("📝 Problem Sets");
                for (i, ps) in week.psets.iter().enumerate() {
                    let key = format!("{}:{}", week.id, i);
                    let done = app.progress.psets_done.contains(&key);
                    ui.horizontal(|ui| {
                        let label = if done {
                            RichText::new("✔").color(GOOD).strong()
                        } else {
                            RichText::new("▢").weak()
                        };
                        if ui.button(label).clicked() {
                            app.pset_pending_explain = Some(key.clone());
                        }
                        let rt = if done {
                            RichText::new(ps).weak().strikethrough()
                        } else {
                            RichText::new(ps).strong()
                        };
                        ui.label(rt);
                    });
                    if let Some(ex) = app.progress.pset_explains.get(&key) {
                        ui.label(
                            RichText::new(format!("   ↳ объяснение: {ex}"))
                                .weak()
                                .size(11.0),
                        );
                    }
                }
                ui.add_space(8.0);
            }

            // ✅ checkpoint
            if !week.checkpoint.is_empty() {
                ui.heading("✅ Чек-пойнт: понял или имитирую?");
                for (i, c) in week.checkpoint.iter().enumerate() {
                    let mut v = app
                        .progress
                        .checkpoint
                        .get(&week.id)
                        .and_then(|v| v.get(i).copied())
                        .unwrap_or(false);
                    if ui.checkbox(&mut v, c).changed() {
                        app.toggle_checkpoint(&week.id, i);
                        app.save();
                    }
                }
                ui.add_space(8.0);
            }

            ui.separator();
            let mut done = app.progress.weeks_done.contains(&week.id);
            if ui
                .checkbox(&mut done, RichText::new("Неделя закрыта (+30 XP)").strong())
                .changed()
            {
                app.toggle_week_done(&week.id);
                app.save();
            }
        });
    });
}
