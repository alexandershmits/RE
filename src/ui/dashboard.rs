use super::theme::{accent, good, info, purple, soft, warn};
use super::widgets::tinted;
use super::{now_and_day, select_tab};
use crate::detector::{self, Severity};
use crate::state::{AppState, Tab};
use crate::util;
use eframe::egui::{self, Color32, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            let (now, today) = now_and_day();
            reexam_banner(app, ui, today);
            ui.heading(
                RichText::new(&app.curriculum.course.title)
                    .color(accent())
                    .size(26.0),
            );
            time_stats(app, ui, now);
            bet_accuracy(app, ui);
            findings(app, ui);
            ui.label(&app.curriculum.course.subtitle);
            ui.add_space(12.0);
            counters(app, ui);
            ui.add_space(12.0);
            next_week(app, ui);
            activity(app, ui, today);
            weak_topics(app, ui);
            data_transfer(app, ui, &ctx);
            course_map(app, ui);
        });
    });
}

fn reexam_banner(app: &mut AppState, ui: &mut egui::Ui, today: u64) {
    if !app.reexam_due(today) {
        return;
    }
    tinted(ui, soft(46, 38, 20), |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🎓 Пора на re-certification!")
                    .color(warn())
                    .size(15.0),
            );
            if ui.small_button("Начать").clicked() {
                app.start_reexam(crate::rng::time_seed());
                select_tab(app, Tab::Reexam);
            }
        });
    });
    ui.add_space(6.0);
}

fn time_stats(app: &AppState, ui: &mut egui::Ui, now: u64) {
    let p = &app.progress;
    let day = util::unix_day(now);
    let by_day = |d: u64| p.time_by_day.get(&d.to_string()).copied().unwrap_or(0);
    let last7: u64 = (day.saturating_sub(6)..=day).map(by_day).sum();
    let total: u64 = p.time_by_day.values().sum::<u64>() + p.pending_seconds;
    let today = by_day(day) + p.pending_seconds;
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!(
            "⏱ Сегодня: {} | 7 дней: {} | Всего: {}",
            util::format_duration(today),
            util::format_duration(last7 + p.pending_seconds),
            util::format_duration(total)
        ))
        .size(13.0)
        .color(info()),
    );
}

fn bet_accuracy(app: &AppState, ui: &mut egui::Ui) {
    let results = &app.progress.bet_results;
    if results.is_empty() {
        return;
    }
    let hit = results.values().filter(|b| **b).count();
    let pct = 100.0 * hit as f32 / results.len() as f32;
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!(
            "🎯 Точность гипотез: {hit}/{} ({pct:.0}%) — {}",
            results.len(),
            if pct >= 60.0 {
                "интуиция калибруется"
            } else {
                "пока угадывание — копай глубже перед ставкой"
            }
        ))
        .size(13.0),
    );
}

fn findings(app: &AppState, ui: &mut egui::Ui) {
    let findings = detector::analyze(&app.progress, app.days_away);
    if findings.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.label(RichText::new("🔎 Поведенческий анализ").strong().size(15.0));
    for f in &findings {
        let color = match f.severity {
            Severity::Info => info(),
            Severity::Warn => warn(),
            Severity::Alert => accent(),
        };
        tinted(ui, soft(40, 40, 30), |ui| {
            ui.label(
                RichText::new(format!("{} {}", f.severity.icon(), f.title))
                    .color(color)
                    .strong(),
            );
            ui.label(RichText::new(&f.advice).size(13.0));
        });
        ui.add_space(4.0);
    }
}

fn counters(app: &AppState, ui: &mut egui::Ui) {
    let p = &app.progress;
    let cards: [(String, &str, Color32); 4] = [
        (
            format!("{}/{}", app.weeks_done_count(), app.weeks_total()),
            "недель закрыто",
            accent(),
        ),
        (
            format!("{}/{}", app.psets_done_count(), app.psets_total()),
            "PSets сдано",
            warn(),
        ),
        (
            format!(
                "{}/{}",
                app.quiz_correct_count(),
                app.curriculum.quizzes.len()
            ),
            "квизов верно",
            good(),
        ),
        (
            format!(
                "{}/{}",
                p.achievements.len(),
                app.curriculum.achievements.len()
            ),
            "ачивок получено",
            purple(),
        ),
    ];
    ui.columns(4, |columns| {
        for (column, (big, small, color)) in columns.iter_mut().zip(cards) {
            column.vertical_centered(|ui| {
                ui.heading(RichText::new(big).color(color).size(24.0));
                ui.label(RichText::new(small).weak().size(11.0));
            });
        }
    });
}

fn next_week(app: &mut AppState, ui: &mut egui::Ui) {
    ui.heading("📌 Следующая неделя");
    let cur = app.curriculum.clone();
    let next = cur
        .weeks
        .iter()
        .position(|w| !app.progress.weeks_done.contains(&w.id))
        .unwrap_or(cur.weeks.len().saturating_sub(1));
    let Some(w) = cur.weeks.get(next) else { return };
    tinted(ui, ui.visuals().faint_bg_color, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.strong(w.label());
            ui.label(&w.title);
            ui.label(RichText::new(cur.module_name(w.module)).weak());
            if ui.button("Открыть →").clicked() {
                app.selected_week = next;
                select_tab(app, Tab::Course);
            }
        });
    });
}

fn activity(app: &AppState, ui: &mut egui::Ui, today: u64) {
    ui.add_space(12.0);
    ui.heading("🔥 Активность");
    let active_today = app.progress.streak.0 == today;
    ui.label(
        RichText::new(format!(
            "Серия: {} дн. {}",
            app.streak_days(today),
            if active_today {
                "(сегодня отмечено ✔)"
            } else {
                "(зайдите сегодня!)"
            }
        ))
        .size(14.0)
        .color(if active_today { good() } else { warn() }),
    );
    let history = &app.progress.xp_history;
    if history.len() < 2 {
        return;
    }
    ui.add_space(4.0);
    ui.label(RichText::new("XP за последние дни:").weak().size(11.0));
    let recent = &history[history.len().saturating_sub(30)..];
    let min = recent.iter().map(|x| x.1).min().unwrap_or(0);
    let max = recent.iter().map(|x| x.1).max().unwrap_or(1);
    let range = (max - min).max(1) as f32;
    ui.horizontal_wrapped(|ui| {
        for (_, xp) in recent {
            let frac = (xp - min) as f32 / range;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 32.0), egui::Sense::hover());
            let bar = egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.bottom() - (4.0 + frac * 28.0)),
                egui::pos2(rect.right(), rect.bottom()),
            );
            ui.painter()
                .rect_filled(bar, 1.0, accent().gamma_multiply(0.4 + frac * 0.6));
        }
    });
}

fn weak_topics(app: &AppState, ui: &mut egui::Ui) {
    ui.add_space(10.0);
    ui.heading("📊 Слабые темы (по неверным ответам квизов)");
    let weak = app.weak_topics();
    if weak.is_empty() {
        ui.label(
            RichText::new("Пока нет данных — пройдите квизы, и я покажу, что подтянуть.").weak(),
        );
    }
    for (name, n) in &weak {
        ui.label(
            RichText::new(format!(
                "🔴 {name}: {n} неверных — повторите их в «Карточках»"
            ))
            .size(13.0),
        );
    }
}

fn data_transfer(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.add_space(10.0);
    ui.heading("📤 Экспорт / импорт прогресса");
    ui.horizontal(|ui| {
        if ui.button("Копировать JSON в буфер").clicked() {
            ui.ctx().copy_text(app.export_progress());
            app.toast("Прогресс скопирован — сохраните в файл!", ctx);
        }
        if ui.button("Импортировать из буфера…").clicked() {
            app.import_dialog = Some(String::new());
        }
    });
    ui.add_space(10.0);
    ui.heading("💾 Перенос профиля");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Экспорт профиля → re50-profile.json").clicked() {
            match app.export_profile_file() {
                Ok(p) => app.toast(format!("Профиль сохранён: {p}"), ctx),
                Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
            }
        }
        if ui.button("Импорт профиля ← re50-profile.json").clicked() {
            match app.import_profile_file() {
                Ok(msg) => {
                    super::apply_theme(ctx, &app.progress);
                    app.toast(msg, ctx)
                }
                Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
            }
        }
    });
    let dir = app
        .paths
        .export_dir
        .as_ref()
        .map_or("домашней папке".to_string(), |p| {
            p.display().to_string()
        });
    ui.label(
        RichText::new(format!(
            "Файл лежит в {dir}. Полный перенос прогресса на другую машину: экспортируйте файл, скопируйте на цель, импортируйте."
        ))
        .weak(),
    );
    ui.add_space(10.0);
    ui.heading("📤 Карточка прогресса");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Сгенерировать карточку прогресса").clicked() {
            app.progress_export_text = format!(
                "🩸 RE-50 | Недели: {}/{} | PSets: {}/{} | Квизы: {}/{} | Ачивки: {}/{} | XP: {} | Прогресс: {}%",
                app.weeks_done_count(),
                app.weeks_total(),
                app.psets_done_count(),
                app.psets_total(),
                app.quiz_correct_count(),
                app.curriculum.quizzes.len(),
                app.progress.achievements.len(),
                app.curriculum.achievements.len(),
                app.progress.xp,
                app.overall_percent() as u32
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
}

fn course_map(app: &AppState, ui: &mut egui::Ui) {
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
            ui.label(RichText::new(format!("({done}/{})", weeks.len())).weak());
        });
        let frac = if weeks.is_empty() {
            0.0
        } else {
            done as f32 / weeks.len() as f32
        };
        ui.add(egui::ProgressBar::new(frac).desired_height(8.0));
    }
}
