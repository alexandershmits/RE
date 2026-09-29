use super::now_and_day;
use super::theme::{accent, good, warn};
use crate::state::{AppState, REEXAM_PASS_PERCENT, REEXAM_QUESTIONS};
use eframe::egui::{self, RichText};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let (_, today) = now_and_day();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading(RichText::new("🎓 Monthly Re-certification").color(accent()));
        ui.add_space(6.0);
        ui.label(format!(
            "Раз в 30 дней приложение устраивает внезапный экзамен: {REEXAM_QUESTIONS} случайных задач из пройденного материала. \
             Порог — {REEXAM_PASS_PERCENT}%. Ошибочные вопросы возвращаются в слабые темы и карточки."
        ));
        ui.add_space(10.0);

        if let Some((date, correct, total)) = &app.progress.reexam_score {
            let passed = correct * 100 >= total * REEXAM_PASS_PERCENT;
            ui.horizontal(|ui| {
                let verdict = if passed { "порог пройден ✔" } else { "ПРОВАЛ — повторите слабые темы" };
                ui.label(RichText::new(format!("Последний результат: {correct}/{total} ({verdict})")).color(if passed { good() } else { warn() }));
                ui.label(RichText::new(date).weak());
            });
            ui.add_space(6.0);
        }

        let Some(rx) = app.reexam.as_ref() else {
            if app.reexam_due(today) {
                let start = egui::Button::new(RichText::new(format!("▶ Начать экзамен ({REEXAM_QUESTIONS} вопросов)")).color(accent()));
                if ui.add(start).clicked() {
                    app.start_reexam(crate::rng::time_seed());
                }
            } else {
                ui.label(RichText::new("⏳ Экзамен пока не назначен — пройдите больше материала.").weak());
            }
            return;
        };
        if rx.finished {
            ui.label("Экзамен завершён. Результат сохранён.");
            if ui.button("Закрыть").clicked() {
                app.reexam = None;
            }
            return;
        }

        let (pos, total, answered, selected) = (rx.pos, rx.questions.len(), rx.answered, rx.selected);
        let q = rx.questions[pos].clone();
        ui.label(format!("Вопрос {} / {total}", pos + 1));
        ui.add_space(4.0);
        ui.label(RichText::new(&q.question).size(16.0));
        ui.add_space(4.0);
        for (i, answer) in q.answers.iter().enumerate() {
            if ui.radio(selected == Some(i), answer).clicked() && !answered {
                if let Some(rx) = app.reexam.as_mut() {
                    rx.selected = Some(i);
                }
            }
        }
        ui.add_space(8.0);
        if !answered {
            if ui.add_enabled(selected.is_some(), egui::Button::new("Ответить")).clicked() {
                if let Some(sel) = selected {
                    app.reexam_answer(sel);
                }
            }
            return;
        }
        if selected == Some(q.correct) {
            ui.label(RichText::new("✔ Верно").color(good()));
        } else {
            ui.label(RichText::new(format!("✖ Неверно. Правильный ответ: {}", q.answers[q.correct])).color(warn()));
        }
        ui.label(RichText::new(&q.explain).weak().size(12.0));
        ui.add_space(6.0);
        let last = pos + 1 >= total;
        if ui.button(if last { "Завершить экзамен" } else { "Далее ▶" }).clicked() {
            if last {
                let message = app.finish_reexam(today);
                let passed = app
                    .progress
                    .reexam_score
                    .as_ref()
                    .is_some_and(|(_, correct, total)| correct * 100 >= total * REEXAM_PASS_PERCENT);
                if passed {
                    app.toast_for(message, &ctx, 8.0);
                } else {
                    app.warn_for(message, &ctx, 8.0);
                }
            } else {
                app.reexam_next();
            }
        }
    });
}
