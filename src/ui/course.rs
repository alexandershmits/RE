use super::theme::{accent, good, soft, warn};
use super::widgets::tinted;
use crate::state::{xp, AppState};
use eframe::egui::{self, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let cur = app.curriculum.clone();
    egui::Panel::right("week_list")
        .exact_size(240.0)
        .show(ui, |ui| {
            ui.heading("Недели");
            ui.separator();
            ScrollArea::vertical().id_salt("weeklist").show(ui, |ui| {
                for (i, w) in cur.weeks.iter().enumerate() {
                    let mark = if app.progress.weeks_done.contains(&w.id) {
                        "✅"
                    } else {
                        "▫"
                    };
                    let label = format!("{mark} {} · {}", w.label(), w.title);
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
        let Some(week) = cur.weeks.get(app.selected_week) else {
            return;
        };
        ScrollArea::vertical()
            .id_salt(("weekdetail", app.selected_week))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading(RichText::new(week.label()).color(accent()));
                    ui.strong(&week.title);
                });
                ui.label(RichText::new(cur.module_name(week.module)).weak());
                if ui
                    .small_button("📂 Открыть лабу недели (→ папка re50-lab)")
                    .clicked()
                {
                    match app.export_week_lab(&week.id) {
                        Ok(dir) => app.toast(format!("TASK.md создан: {dir}"), &ctx),
                        Err(e) => app.toast(format!("Не удалось создать лабу: {e}"), &ctx),
                    }
                }
                ui.separator();

                if let Some(case) = &week.case {
                    tinted(ui, soft(38, 30, 40), |ui| {
                        ui.strong(
                            RichText::new("🎯 Проблема недели — сначала ЗАЧЕМ, потом КАК:")
                                .color(warn()),
                        );
                        super::markup::show(ui, case);
                    });
                    ui.add_space(8.0);
                }

                if !week.lectures.is_empty() {
                    ui.heading("📖 Лекции");
                    for l in &week.lectures {
                        super::markup::show(ui, &format!("• {l}"));
                    }
                    ui.add_space(8.0);
                }

                if let Some(lab) = &week.lab {
                    ui.heading(format!("🧪 {}", lab.title));
                    for (i, step) in lab.steps.iter().enumerate() {
                        let key = format!("{}:{i}", week.id);
                        let mut checked = app.progress.lab_steps_done.contains(&key);
                        if ui.checkbox(&mut checked, step).changed() {
                            app.toggle_lab_step(&key);
                        }
                    }
                    ui.add_space(8.0);
                }

                if !week.psets.is_empty() {
                    ui.heading("📝 Problem Sets");
                    for (i, ps) in week.psets.iter().enumerate() {
                        let key = format!("{}:{i}", week.id);
                        let done = app.progress.psets_done.contains(&key);
                        ui.horizontal_wrapped(|ui| {
                            let mark = if done {
                                RichText::new("✔").color(good()).strong()
                            } else {
                                RichText::new("▢").weak()
                            };
                            if ui
                                .button(mark)
                                .on_hover_text("Сдать с объяснением своими словами")
                                .clicked()
                            {
                                app.pset_draft = app
                                    .progress
                                    .pset_explains
                                    .get(&key)
                                    .cloned()
                                    .unwrap_or_default();
                                app.pset_pending_explain = Some(key.clone());
                            }
                            let text = if done {
                                RichText::new(ps).weak().strikethrough()
                            } else {
                                RichText::new(ps).strong()
                            };
                            ui.label(text);
                        });
                        if let Some(explanation) = app.progress.pset_explains.get(&key) {
                            ui.label(
                                RichText::new(format!("   ↳ объяснение: {explanation}"))
                                    .weak()
                                    .size(11.0),
                            );
                        }
                    }
                    ui.add_space(8.0);
                }

                if !week.checkpoint.is_empty() {
                    ui.heading("✅ Чек-пойнт: понял или имитирую?");
                    for (i, item) in week.checkpoint.iter().enumerate() {
                        let mut checked = app.checkpoint_checked(&week.id, i);
                        if ui.checkbox(&mut checked, item).changed() {
                            app.toggle_checkpoint(&week.id, i);
                        }
                    }
                    ui.add_space(8.0);
                }

                ui.separator();
                let mut done = app.progress.weeks_done.contains(&week.id);
                let label =
                    RichText::new(format!("Неделя закрыта (+{} XP, один раз)", xp::WEEK)).strong();
                if ui.checkbox(&mut done, label).changed() {
                    app.toggle_week_done(&week.id);
                }
            });
    });
}
