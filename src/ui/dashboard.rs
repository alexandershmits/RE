use eframe::egui::{self, Color32, RichText, ScrollArea};

use super::{ACCENT, GOOD, WARN};
use crate::state::{AppState, Tab};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = &ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            // Внезапный экзамен: баннер, когда срок подошёл
            {
                let day = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() / 86400)
                    .unwrap_or(0);
                if app.reexam_due(day) {
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(46, 38, 20))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🎓 Пора на re-certification!").color(WARN).size(15.0));
                                if ui.small_button("Начать").clicked() {
                                    let seed = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(1);
                                    app.start_reexam(seed);
                                    app.tab = Tab::Reexam;
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            }
            ui.heading(RichText::new(&app.curriculum.course.title).color(ACCENT).size(26.0));

            // Поведенческий анализ (анти-паттерны)
            // Статистика времени
            {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let today = (now / 86400).to_string();
                let mut last7 = 0u64;
                for d in (now / 86400).saturating_sub(6)..=(now / 86400) {
                    last7 += app.progress.time_by_day.get(&d.to_string()).copied().unwrap_or(0);
                }
                let total: u64 = app.progress.time_by_day.values().sum();
                let today_s = app.progress.time_by_day.get(&today).copied().unwrap_or(0) + app.progress.pending_seconds;
                let fmt = |s: u64| if s >= 3600 { format!("{:.1} ч", s as f32 / 3600.0) } else { format!("{} мин", s / 60) };
                ui.add_space(4.0);
                ui.label(RichText::new(format!(
                    "⏱ Сегодня: {} | 7 дней: {} | Всего: {}",
                    fmt(today_s), fmt(last7), fmt(total)
                )).size(13.0).color(egui::Color32::from_rgb(120, 170, 230)));
            }
            // Точность интуиции (ставки)
            if !app.progress.bet_results.is_empty() {
                let total = app.progress.bet_results.len();
                let hit = app.progress.bet_results.values().filter(|b| **b).count();
                let pct = 100.0 * hit as f32 / total as f32;
                ui.add_space(6.0);
                ui.label(RichText::new(format!(
                    "🎯 Точность гипотез: {hit}/{total} ({pct:.0}%) — {}",
                    if pct >= 60.0 { "интуиция калибруется" } else { "пока угадывание — копай глубже перед ставкой" }
                )).size(13.0));
            }
            {
                let findings = crate::detector::analyze(&app.progress, &app.curriculum);
                if !findings.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("🔎 Поведенческий анализ").strong().size(15.0));
                    for f in &findings {
                        egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(40, 40, 30)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} {}", f.severity.icon(), f.title))
                                    .color(f.severity.color32()).strong());
                            });
                            ui.label(RichText::new(&f.advice).size(13.0));
                        });
                        ui.add_space(4.0);
                    }
                }
            }
            ui.label(&app.curriculum.course.subtitle);
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                for (big, small, color) in [
                    (
                        format!("{}/{}", app.weeks_done_count(), app.weeks_total()),
                        "недель закрыто",
                        ACCENT,
                    ),
                    (
                        format!("{}/{}", app.psets_done_count(), app.psets_total()),
                        "PSets сдано",
                        WARN,
                    ),
                    (
                        format!(
                            "{}/{}",
                            app.progress.quiz_correct.len(),
                            app.curriculum.quizzes.len()
                        ),
                        "квизов верно",
                        GOOD,
                    ),
                    (
                        format!(
                            "{}/{}",
                            app.progress.achievements.len(),
                            app.curriculum.achievements.len()
                        ),
                        "ачивок получено",
                        Color32::from_rgb(200, 140, 255),
                    ),
                ] {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new(big).color(color).size(24.0));
                        ui.label(RichText::new(small).weak().size(11.0));
                    });
                }
            });
            ui.add_space(12.0);

            ui.heading("📌 Следующая неделя");
            let next = app
                .curriculum
                .weeks
                .iter()
                .position(|w| !app.progress.weeks_done.contains(&w.id))
                .unwrap_or(app.curriculum.weeks.len().saturating_sub(1));
            let w = &app.curriculum.weeks[next];
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(w.label());
                    ui.label(&w.title);
                    ui.label(RichText::new(app.curriculum.module_name(w.module)).weak());
                    if ui.button("Открыть →").clicked() {
                        app.selected_week = next;
                        app.tab = Tab::Course;
                    }
                });
            });
            ui.add_space(12.0);

            ui.heading("🔥 Активность");
            let (last_day, streak) = app.progress.streak;
            let today = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            let active_today = last_day == today;
            ui.label(RichText::new(format!(
                "Серия: {} дн. {}",
                streak,
                if active_today { "(сегодня отмечено ✓)" } else { "(зайдите сегодня!)" }
            )).size(14.0).color(if active_today { GOOD } else { WARN }));

            // XP sparkline (simple bars, last 30 days)
            let h = &app.progress.xp_history;
            if h.len() >= 2 {
                ui.add_space(4.0);
                ui.label(RichText::new("XP за последние дни:").weak().size(11.0));
                let recent: Vec<(u64, u32)> = h[h.len().saturating_sub(30)..].to_vec();
                let min = recent.iter().map(|x| x.1).min().unwrap_or(0);
                let max = recent.iter().map(|x| x.1).max().unwrap_or(1);
                let range = (max - min).max(1) as f32;
                ui.horizontal_wrapped(|ui| {
                    for (_day, xp) in &recent {
                        let frac = (*xp - min) as f32 / range;
                        let hgt = 4.0 + frac * 28.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(6.0, 32.0),
                            egui::Sense::hover(),
                        );
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - hgt),
                            egui::pos2(rect.right(), rect.bottom()),
                        );
                        ui.painter().rect_filled(bar, 1.0, ACCENT.gamma_multiply(0.4 + frac * 0.6));
                    }
                });
            }
            ui.add_space(10.0);

            ui.heading("📊 Слабые темы (по неверным ответам квизов)");
            let weak = app.weak_topics();
            if weak.is_empty() {
                ui.label(RichText::new("Пока нет данных — пройдите квизы, и я покажу, что подтянуть.").weak());
            } else {
                for (name, n) in &weak {
                    ui.label(RichText::new(format!("🔴 {name}: {n} неверных — перейдите в карточки и дриллы этой темы")).size(13.0));
                }
            }
            ui.add_space(10.0);

            ui.heading("📤 Экспорт / импорт прогресса");
            ui.horizontal(|ui| {
                if ui.button("Копировать JSON в буфер").clicked() {
                    ui.ctx().copy_text(app.export_progress());
                    app.toast("Прогресс скопирован — сохраните в файл!", ctx);
                }
                if ui.button("Импортировать из буфера").clicked() {
                    let txt = ui.ctx().input(|i| i.events.iter().find_map(|e| match e {
                        egui::Event::Paste(s) => Some(s.clone()), _ => None }).unwrap_or_default());
                    if txt.is_empty() {
                        app.toast("Скопируйте JSON в буфер (Ctrl+C), затем нажмите импорт", ctx);
                    } else if app.import_progress(&txt) {
                        app.toast("Прогресс импортирован!", ctx);
                    } else {
                        app.toast("Ошибка: некорректный JSON", ctx);
                    }
                }
            });
            ui.add_space(10.0);

            ui.heading("💾 Перенос профиля");
            ui.horizontal(|ui| {
                if ui.button("Экспорт профиля → ~/re50-profile.json").clicked() {
                    match app.export_profile_file() {
                        Ok(p) => app.toast(format!("Профиль сохранён: {p}"), ctx),
                        Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
                    }
                }
                if ui.button("Импорт профиля ← ~/re50-profile.json").clicked() {
                    match app.import_profile_file() {
                        Ok(msg) => app.toast(msg, ctx),
                        Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
                    }
                }
            });
            ui.label(RichText::new("Полный перенос прогресса на другую машину: экспортируйте файл, скопируйте на цель, импортируйте.").weak().size(12.0));
            ui.add_space(10.0);

            ui.heading("📤 Карточка прогресса");
            ui.horizontal(|ui| {
                if ui.button("Сгенерировать карточку прогресса").clicked() {
                    let pct = app.overall_percent() as u32;
                    app.progress_export_text = format!(
                        "🩸 RE-50 | Недели: {}/{} | PSets: {}/{} | Квизы: {}/{} | Ачивки: {}/{} | XP: {} | Прогресс: {}%",
                        app.weeks_done_count(), app.weeks_total(),
                        app.psets_done_count(), app.psets_total(),
                        app.progress.quiz_correct.len(), app.curriculum.quizzes.len(),
                        app.progress.achievements.len(), app.curriculum.achievements.len(),
                        app.progress.xp, pct
                    );
                }
                if !app.progress_export_text.is_empty() {
                    ui.monospace(&app.progress_export_text);
                    if ui.button("📋").on_hover_text("Скопировать").clicked() {
                        ui.ctx().copy_text(app.progress_export_text.clone());
                        app.toast("Скопировано в буфер обмена", ctx);
                    }
                }
            });
            ui.add_space(12.0);

            ui.heading("🗺 Карта курса");
            for m in &app.curriculum.modules {
                let weeks: Vec<_> = app
                    .curriculum
                    .weeks
                    .iter()
                    .filter(|w| w.module == m.id)
                    .collect();
                let done = weeks
                    .iter()
                    .filter(|w| app.progress.weeks_done.contains(&w.id))
                    .count();
                ui.horizontal(|ui| {
                    ui.strong(format!("{} {}", m.icon, m.name));
                    ui.label(RichText::new(format!("({}/{})", done, weeks.len())).weak());
                });
                ui.add(
                    egui::ProgressBar::new(if weeks.is_empty() { 0.0 } else { done as f32 / weeks.len() as f32 })
                        .desired_height(8.0),
                );
            }
        });
    });
}
