use eframe::egui::{self, Color32, RichText, ScrollArea};

use crate::state::{AppState, Tab};

pub(crate) const ACCENT: Color32 = Color32::from_rgb(220, 60, 70); // 🩸 blood red
pub(crate) const GOOD: Color32 = Color32::from_rgb(90, 200, 120);
pub(crate) const WARN: Color32 = Color32::from_rgb(240, 180, 70);

mod achievements;
mod cards;
mod challenges;
mod course;
mod dashboard;
mod diagrams;
mod drills;
mod interview;
mod journal;
mod opponent;
mod placement;
mod reexam;
mod resources;
mod rubric;
mod sims;
mod trainer;
mod work;

pub fn run(app: &mut AppState, root: &mut egui::Ui) {
    let ctx = &root.ctx().clone();
    // Горячие клавиши: Ctrl+K — фокус поиска, Ctrl+1..9 — вкладки, Ctrl+J — журнал
    ctx.input(|i| {
        if i.modifiers.command {
            let keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
                egui::Key::Num6,
                egui::Key::Num7,
                egui::Key::Num8,
                egui::Key::Num9,
            ];
            let tabs = [
                Tab::Dashboard,
                Tab::Course,
                Tab::Trainer,
                Tab::Achievements,
                Tab::Resources,
                Tab::Journal,
                Tab::Cards,
                Tab::Drills,
                Tab::Challenges,
            ];
            for (key, tab) in keys.iter().zip(tabs.iter()) {
                if i.key_pressed(*key) {
                    app.tab = *tab;
                }
            }
            if i.key_pressed(egui::Key::J) {
                app.tab = Tab::Journal;
            }
            if i.key_pressed(egui::Key::K) {
                app.search_focus = true;
            }
        }
    });
    // toast
    if let Some((_msg, until)) = app.toast.clone() {
        if app.now(ctx) > until {
            app.toast = None;
        } else if let Some((m, _)) = &app.toast {
            egui::Area::new(egui::Id::new("toast"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 12.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(Color32::from_rgb(40, 44, 56))
                        .stroke(egui::Stroke::new(1.0_f32, ACCENT))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.colored_label(GOOD, format!("✔ {m}"));
                        });
                });
        }
    }

    // PSet self-explain modal
    if let Some(key) = app.pset_pending_explain.clone() {
        egui::Window::new("📝 Правило честности курса")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("Объясните решение своими словами (2–3 предложения). Не можете объяснить — значит, пока не решили. Это защита от самообмана.").weak());
                ui.add_space(6.0);
                let mut text = app
                    .progress
                    .pset_explains
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();
                ui.add_sized([420.0, 100.0], egui::TextEdit::multiline(&mut text));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✔ Сдать").strong()).clicked() {
                        let k = key.clone();
                        app.pset_pending_explain = None;
                        app.complete_pset_with_explain(&k, text.clone());
                        app.save();
                    }
                    if ui.button("Отмена").clicked() {
                        app.pset_pending_explain = None;
                    }
                });
            });
    }

    // achievement popups
    if !app.new_achievements.is_empty() {
        let name = app.new_achievements[0].clone();
        egui::Area::new(egui::Id::new("ach_popup"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(46, 38, 20))
                    .stroke(egui::Stroke::new(1.5_f32, WARN))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.strong(RichText::new("🏅 НОВАЯ АЧИВКА!").color(WARN).size(16.0));
                            ui.label(&name);
                            if ui.button("Отлично!").clicked() {
                                app.new_achievements.remove(0);
                            }
                        });
                    });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    egui::Panel::top("topbar").show(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🩸 RE-50").color(ACCENT).size(22.0));
            ui.label(
                RichText::new(&app.curriculum.course.subtitle)
                    .weak()
                    .size(12.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Глобальный поиск
                let editor = egui::TextEdit::singleline(&mut app.search_query)
                    .hint_text("🔍 Поиск (Ctrl+K)")
                    .desired_width(200.0)
                    .id(egui::Id::new("global_search"));
                let resp = ui.add(editor);
                if app.search_focus {
                    resp.request_focus();
                    app.search_focus = false;
                }
                if resp.changed() {
                    app.search_results = app.search_course(&app.search_query);
                }
                // Настройки вида
                if ui
                    .small_button("🌓")
                    .on_hover_text("Светлая/тёмная тема")
                    .clicked()
                {
                    app.progress.theme = if app.progress.theme == "light" {
                        "dark".into()
                    } else {
                        "light".into()
                    };
                    app.apply_style(ui.ctx());
                    app.save();
                }
                if ui
                    .small_button("A−")
                    .on_hover_text("Меньше шрифт")
                    .clicked()
                {
                    app.progress.font_scale = (app.progress.font_scale - 0.1).max(0.8);
                    app.apply_style(ui.ctx());
                    app.save();
                }
                if ui
                    .small_button("A+")
                    .on_hover_text("Больше шрифт")
                    .clicked()
                {
                    app.progress.font_scale = (app.progress.font_scale + 0.1).min(2.0);
                    app.apply_style(ui.ctx());
                    app.save();
                }
                ui.label(RichText::new(format!("⭐ {} XP", app.progress.xp)).color(WARN));
                let pct = app.overall_percent();
                ui.add(
                    egui::ProgressBar::new(pct / 100.0)
                        .text(format!("{:.0}% курса", pct))
                        .desired_width(160.0),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();
    });

    egui::Panel::left("nav").exact_size(190.0).show(root, |ui| {
        ui.add_space(8.0);
        for (tab, icon, label) in [
            (Tab::Dashboard, "🏠", "Дашборд"),
            (Tab::Course, "📚", "Курс"),
            (Tab::Trainer, "🎯", "Тренажёр"),
            (Tab::Achievements, "🏅", "Ачивки"),
            (Tab::Resources, "🔗", "Ресурсы"),
            (Tab::Journal, "📓", "Журнал"),
            (Tab::Cards, "🃏", "Карточки"),
            (Tab::Interview, "🎤", "Интервью"),
            (Tab::Rubric, "📋", "Rubric отчёта"),
            (Tab::Diagrams, "🗺", "Схемы"),
            (Tab::Placement, "🧪", "Тест входа"),
            (Tab::Sims, "⚙️", "Симуляторы"),
            (Tab::Drills, "🔁", "Дриллы (89)"),
            (Tab::Challenges, "🚩", "Челленджи (15)"),
            (Tab::Reexam, "🎓", "Re-certification"),
            (Tab::Opponent, "🥋", "Оппонент"),
            (Tab::Work, "💼", "Рабочая сессия"),
        ] {
            let selected = app.tab == tab;
            if ui
                .selectable_label(selected, format!("{icon}  {label}"))
                .clicked()
            {
                app.tab = tab;
                if app.tab == Tab::Trainer {
                    app.quiz = None;
                }
            }
        }
        ui.separator();
        ui.label(RichText::new("Правила курса:").weak());
        ScrollArea::vertical().show(ui, |ui| {
            for r in &app.curriculum.course.rules {
                ui.label(RichText::new(format!("• {r}")).size(11.0).weak());
            }
        });
    });

    let tab = std::mem::take(&mut app.tab);
    match tab {
        Tab::Dashboard => dashboard::show(app, root),
        Tab::Course => course::show(app, root),
        Tab::Trainer => trainer::show(app, root),
        Tab::Achievements => achievements::show(app, root),
        Tab::Resources => resources::show(app, root),
        Tab::Journal => journal::show(app, root),
        Tab::Cards => cards::show(app, root),
        Tab::Interview => interview::show(app, root),
        Tab::Rubric => rubric::show(app, root),
        Tab::Diagrams => diagrams::show(app, root),
        Tab::Placement => placement::show(app, root),
        Tab::Sims => sims::show(app, root),
        Tab::Drills => drills::show(app, root),
        Tab::Challenges => challenges::show(app, root),
        Tab::Reexam => reexam::show(app, root),
        Tab::Opponent => opponent::show(app, root),
        Tab::Work => work::show(app, root),
    }
    app.tab = tab;

    // Окно результатов поиска
    if !app.search_query.trim().is_empty() {
        let mut close = false;
        let mut jump: Option<(String, String)> = None;
        let results = app.search_results.clone();
        egui::Window::new("🔍 Результаты поиска")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
            .show(ctx, |ui| {
                if results.is_empty() {
                    ui.label(RichText::new("Ничего не найдено").weak());
                }
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (title, kind, wid) in &results {
                            if ui.link(title).clicked() {
                                jump = Some((kind.clone(), wid.clone()));
                            }
                        }
                    });
                if ui.button("Закрыть").clicked() {
                    close = true;
                }
            });
        if let Some((kind, wid)) = jump {
            match kind.as_str() {
                "challenge" => {
                    app.tab = Tab::Challenges;
                }
                "quiz" => {
                    app.tab = Tab::Trainer;
                }
                "resource" => {
                    app.tab = Tab::Resources;
                }
                _ => {
                    // переход к неделе
                    if let Some(idx) = app.curriculum.weeks.iter().position(|w| w.id == wid) {
                        app.selected_week = idx;
                        app.tab = Tab::Course;
                    }
                }
            }
            close = true;
        }
        if close {
            app.search_query.clear();
            app.search_results.clear();
        }
    }
}
