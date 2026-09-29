//! Интерфейс: корневая раскладка, навигация, всплывающие окна.

use crate::jobs::JobState;
use crate::state::progress::{MAX_FONT_SCALE, MIN_FONT_SCALE};
use crate::state::{AppState, HitKind, Tab};
use crate::util;
use eframe::egui::{self, RichText, ScrollArea};
use std::time::Duration;

mod achievements;
mod cards;
mod challenges;
mod course;
mod dashboard;
mod diagrams;
mod drills;
mod interview;
mod journal;
mod markup;
mod opponent;
mod placement;
mod reexam;
mod resources;
mod rubric;
mod sims;
pub mod theme;
mod trainer;
pub mod widgets;
mod work;

use theme::{accent, good, soft, warn};
/// Сколько секунд висит окно новой ачивки, если его не закрыли.
const POPUP_SECS: f64 = 8.0;

pub use theme::{apply as apply_theme, install};

/// Один кадр всего интерфейса.
pub fn run(app: &mut AppState, root: &mut egui::Ui) {
    let ctx = &root.ctx().clone();
    hotkeys(app, ctx);
    poll_background(app, ctx);
    notices(app, ctx);
    pset_modal(app, ctx);
    import_window(app, ctx);
    achievement_popup(app, ctx);
    top_bar(app, root);
    nav_panel(app, root);
    match app.tab {
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
    search_window(app, ctx);
}

/// Ctrl+1..9 — вкладки, Ctrl+J — журнал, Ctrl+K — поиск.
fn hotkeys(app: &mut AppState, ctx: &egui::Context) {
    const KEYS: [egui::Key; 9] = [
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
    const TABS: [Tab; 9] = [
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
    let (target, search) = ctx.input(|i| {
        if !i.modifiers.command {
            return (None, false);
        }
        let tab = if i.key_pressed(egui::Key::J) {
            Some(Tab::Journal)
        } else {
            KEYS.iter()
                .zip(TABS)
                .find(|(k, _)| i.key_pressed(**k))
                .map(|(_, t)| t)
        };
        (tab, i.key_pressed(egui::Key::K))
    });
    if let Some(tab) = target {
        select_tab(app, tab);
    }
    app.search_focus |= search;
}

fn select_tab(app: &mut AppState, tab: Tab) {
    app.tab = tab;
    if tab == Tab::Trainer {
        app.quiz = None;
    }
}

/// Результаты фоновых задач приходят в любой вкладке.
fn poll_background(app: &mut AppState, ctx: &egui::Context) {
    let state = app.generator.as_ref().map(|job| job.poll());
    match state {
        Some(JobState::Running) => ctx.request_repaint_after(Duration::from_millis(200)),
        Some(JobState::Done(report)) => {
            app.generator = None;
            app.toast_for(report.message, ctx, 12.0);
        }
        Some(JobState::Failed) => {
            app.generator = None;
            app.toast(
                "Ошибка генератора: фоновая задача завершилась аварийно",
                ctx,
            );
        }
        None => {}
    }
    opponent::poll(app, ctx);
}

/// Уведомление о восстановлении, ошибка сохранения и всплывающие сообщения.
fn notices(app: &mut AppState, ctx: &egui::Context) {
    if let Some(text) = app.startup_notice.take() {
        app.toast_for(text, ctx, 12.0);
    }
    if let Some(error) = app.save_error.clone() {
        egui::Area::new(egui::Id::new("save_error"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(soft(60, 24, 28))
                    .stroke(egui::Stroke::new(1.0, accent()))
                    .show(ui, |ui| {
                        ui.colored_label(accent(), format!("⚠ Прогресс не сохраняется: {error}"));
                    });
            });
    }
    let Some((message, until)) = app.toast.clone() else {
        return;
    };
    if app.now(ctx) > until {
        app.toast = None;
        return;
    }
    let failed = message.starts_with("Ошибка") || message.starts_with("Не удалось");
    egui::Area::new(egui::Id::new("toast"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 12.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(soft(40, 44, 56))
                .stroke(egui::Stroke::new(1.0, accent()))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_max_width(640.0);
                    if failed {
                        ui.colored_label(warn(), format!("⚠ {message}"));
                    } else {
                        ui.colored_label(good(), format!("✔ {message}"));
                    }
                });
        });
    ctx.request_repaint_after(Duration::from_millis(250));
}

/// Правило честности: PSet засчитывается вместе с объяснением своими словами.
fn pset_modal(app: &mut AppState, ctx: &egui::Context) {
    let Some(key) = app.pset_pending_explain.clone() else {
        return;
    };
    let (mut submit, mut cancel) = (false, false);
    egui::Window::new("📝 Правило честности курса")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(RichText::new("Объясните решение своими словами (2–3 предложения). Не можете объяснить — значит, пока не решили. Это защита от самообмана.").weak());
            ui.add_space(6.0);
            ui.add_sized([420.0, 100.0], egui::TextEdit::multiline(&mut app.pset_draft));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let filled = !app.pset_draft.trim().is_empty();
                submit = ui.add_enabled(filled, egui::Button::new(RichText::new("✔ Сдать").strong())).clicked();
                cancel = ui.button("Отмена").clicked();
            });
        });
    if submit {
        let text = std::mem::take(&mut app.pset_draft);
        app.complete_pset_with_explain(&key, text);
    } else if cancel {
        app.pset_pending_explain = None;
        app.pset_draft.clear();
    }
}

/// Импорт прогресса из буфера обмена: вставьте JSON и нажмите «Импортировать».
fn import_window(app: &mut AppState, ctx: &egui::Context) {
    let Some(mut text) = app.import_dialog.take() else {
        return;
    };
    let (mut import, mut cancel) = (false, false);
    egui::Window::new("📥 Импорт прогресса")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label("Вставьте сюда JSON прогресса (Ctrl+V). Текущий прогресс сохранится в резервной копии.");
            ui.add_sized([520.0, 160.0], egui::TextEdit::multiline(&mut text).code_editor());
            ui.horizontal(|ui| {
                import = ui.add_enabled(!text.trim().is_empty(), egui::Button::new("Импортировать")).clicked();
                cancel = ui.button("Отмена").clicked();
            });
        });
    if import {
        match app.import_progress(&text) {
            Ok(()) => app.toast("Прогресс импортирован", ctx),
            Err(e) => {
                app.toast(format!("Ошибка: {e}"), ctx);
                app.import_dialog = Some(text);
            }
        }
    } else if !cancel {
        app.import_dialog = Some(text);
    }
}

fn achievement_popup(app: &mut AppState, ctx: &egui::Context) {
    let Some(name) = app.new_achievements.first().cloned() else {
        app.popup_until = None;
        return;
    };
    let now = app.now(ctx);
    let until = *app.popup_until.get_or_insert(now + POPUP_SECS);
    let mut dismiss = now > until;
    if !dismiss {
        egui::Area::new(egui::Id::new("ach_popup"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(soft(46, 38, 20))
                    .stroke(egui::Stroke::new(1.5, warn()))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.strong(RichText::new("🏅 НОВАЯ АЧИВКА!").color(warn()).size(16.0));
                        ui.label(&name);
                        dismiss = ui.button("Отлично!").clicked();
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(500));
    }
    if dismiss {
        app.new_achievements.remove(0);
        app.popup_until = None;
    }
}

fn top_bar(app: &mut AppState, root: &mut egui::Ui) {
    egui::Panel::top("topbar").show(root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🩸 RE-50").color(accent()).size(22.0));
            if ui.available_width() > 1150.0 {
                ui.label(
                    RichText::new(&app.curriculum.course.subtitle)
                        .weak()
                        .size(12.0),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let editor = egui::TextEdit::singleline(&mut app.search_query)
                    .hint_text("🔍 Поиск (Ctrl+K)")
                    .desired_width(200.0)
                    .id(egui::Id::new("global_search"));
                let response = ui.add(editor);
                if std::mem::take(&mut app.search_focus) {
                    response.request_focus();
                }
                if response.changed() {
                    app.search_results = app.search_course(&app.search_query);
                }
                if ui
                    .small_button("🌓")
                    .on_hover_text("Светлая/тёмная тема")
                    .clicked()
                {
                    app.progress.theme = if app.progress.is_light() {
                        "dark"
                    } else {
                        "light"
                    }
                    .into();
                    apply_theme(ui.ctx(), &app.progress);
                    app.mark_dirty();
                }
                for (label, hint, step) in
                    [("A+", "Больше шрифт", 0.1), ("A−", "Меньше шрифт", -0.1)]
                {
                    if ui.small_button(label).on_hover_text(hint).clicked() {
                        let scale = ((app.progress.font_scale + step) * 10.0).round() / 10.0;
                        app.progress.font_scale = scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE);
                        apply_theme(ui.ctx(), &app.progress);
                        app.mark_dirty();
                    }
                }
                ui.label(RichText::new(format!("⭐ {} XP", app.progress.xp)).color(warn()));
                let pct = app.overall_percent();
                ui.add(
                    egui::ProgressBar::new(pct / 100.0)
                        .text(format!("{pct:.0}% курса"))
                        .desired_width(150.0),
                );
            });
        });
        ui.add_space(4.0);
    });
}

fn nav_panel(app: &mut AppState, root: &mut egui::Ui) {
    let d = &app.curriculum.drills;
    let drills = d.asm.len() + d.addr.len() + d.pattern.len() + d.script.len();
    let challenges = app.curriculum.challenges.len();
    let items = [
        (Tab::Dashboard, "🏠", "Дашборд".to_string()),
        (Tab::Course, "📚", "Курс".to_string()),
        (Tab::Trainer, "🎯", "Тренажёр".to_string()),
        (Tab::Achievements, "🏅", "Ачивки".to_string()),
        (Tab::Resources, "🔗", "Ресурсы".to_string()),
        (Tab::Journal, "📓", "Журнал".to_string()),
        (Tab::Cards, "🃏", "Карточки".to_string()),
        (Tab::Interview, "🎤", "Интервью".to_string()),
        (Tab::Rubric, "📋", "Rubric отчёта".to_string()),
        (Tab::Diagrams, "🗺", "Схемы".to_string()),
        (Tab::Placement, "🧪", "Тест входа".to_string()),
        (Tab::Sims, "⚙", "Симуляторы".to_string()),
        (Tab::Drills, "🔁", format!("Дриллы ({drills})")),
        (Tab::Challenges, "🚩", format!("Челленджи ({challenges})")),
        (Tab::Reexam, "🎓", "Re-certification".to_string()),
        (Tab::Opponent, "🥋", "Оппонент".to_string()),
        (Tab::Work, "💼", "Рабочая сессия".to_string()),
    ];
    egui::Panel::left("nav").exact_size(190.0).show(root, |ui| {
        ui.add_space(8.0);
        for (tab, icon, label) in items {
            if ui
                .selectable_label(app.tab == tab, format!("{icon}  {label}"))
                .clicked()
            {
                select_tab(app, tab);
            }
        }
        ui.separator();
        ui.label(RichText::new("Правила курса:").weak());
        ScrollArea::vertical().show(ui, |ui| {
            for rule in &app.curriculum.course.rules {
                ui.label(RichText::new(format!("• {rule}")).size(11.0).weak());
            }
        });
    });
}

fn search_window(app: &mut AppState, ctx: &egui::Context) {
    if app.search_query.trim().is_empty() {
        return;
    }
    let (mut close, mut jump) = (false, None);
    egui::Window::new("🔍 Результаты поиска")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 80.0])
        .show(ctx, |ui| {
            if app.search_results.is_empty() {
                ui.label(RichText::new("Ничего не найдено").weak());
            }
            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for hit in &app.search_results {
                    if ui.link(&hit.title).clicked() {
                        jump = Some((hit.kind, hit.week_id.clone()));
                    }
                }
            });
            close = ui.button("Закрыть").clicked();
        });
    if let Some((kind, week_id)) = jump {
        match kind {
            HitKind::Challenge => app.tab = Tab::Challenges,
            HitKind::Quiz => select_tab(app, Tab::Trainer),
            HitKind::Resource => app.tab = Tab::Resources,
            HitKind::Week => {
                if let Some(idx) = app.curriculum.weeks.iter().position(|w| w.id == week_id) {
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

/// Текущий момент для тех вкладок, которым нужны секунды и номер дня.
pub(crate) fn now_and_day() -> (u64, u64) {
    let now = util::unix_now();
    (now, util::unix_day(now))
}
