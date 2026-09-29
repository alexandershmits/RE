use eframe::egui::{self, RichText};

use super::{ACCENT, GOOD, WARN};

pub(super) fn show(app: &mut crate::state::AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading(RichText::new("🎓 Monthly Re-certification").color(ACCENT));
        ui.add_space(6.0);
        ui.label("Раз в 30 дней приложение устраивает внезапный экзамен: 10 случайных задач из пройденного материала. Порог — 70%. Провал → темы возвращаются в слабые.");
        ui.add_space(10.0);

        if let Some(score) = &app.progress.reexam_score {
            let (date, c, t) = score;
            let passed = (*c as f32) >= 0.7 * (*t as f32);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(
                    "Последний результат: {c}/{t} ({})",
                    if passed { "порог пройден ✔" } else { "ПРОВАЛ — повторите слабые темы" }
                )).color(if passed { GOOD } else { WARN }));
                ui.label(RichText::new(date).weak());
            });
            ui.add_space(6.0);
        }

        let due = {
            let day = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            app.reexam_due(day)
        };

        if app.reexam.is_none() {
            if due {
                if ui.add(egui::Button::new(RichText::new("▶ Начать экзамен (10 вопросов)").color(ACCENT))).clicked() {
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(1);
                    app.start_reexam(seed);
                }
            } else {
                ui.label(RichText::new("⏳ Экзамен пока не назначен — пройдите больше материала.").weak());
            }
            return;
        }

        let Some(rx) = app.reexam.as_mut() else { return; };
        if rx.finished {
            ui.label("Экзамен завершён. Результат сохранён.");
            if ui.button("Закрыть").clicked() {
                app.reexam = None;
            }
            return;
        }

        let total = rx.questions.len();
        ui.label(format!("Вопрос {} / {}", rx.pos + 1, total));
        let q = &rx.questions[rx.pos];
        ui.add_space(4.0);
        ui.label(RichText::new(&q.question).size(16.0));
        ui.add_space(4.0);
        for (i, a) in q.answers.iter().enumerate() {
            let is_sel = rx.selected == Some(i);
            if ui.add(egui::RadioButton::new(is_sel, a)).clicked() && !rx.answered {
                rx.selected = Some(i);
            }
        }
        ui.add_space(8.0);
        if !rx.answered {
            if ui.add(egui::Button::new("Ответить")).clicked() {
                if let Some(sel) = rx.selected {
                    rx.answered = true;
                    if sel == q.correct { rx.correct += 1; }
                }
            }
        } else {
            let sel_ok = rx.selected == Some(q.correct);
            ui.label(
                RichText::new(if sel_ok { "✔ Верно".to_string() } else {
                    format!("✘ Неверно. Правильный ответ: {}", q.answers[q.correct])
                })
                .color(if sel_ok { GOOD } else { WARN }),
            );
            ui.label(RichText::new(&q.explain).weak().size(12.0));
            ui.add_space(6.0);
            let last = rx.pos + 1 >= total;
            let date = {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                // days since epoch -> yyyy-mm-dd approx
                let days = secs / 86400;
                let y = 1970 + days / 365;
                format!("re-exam day {days} (~{y})")
            };
            if ui.add(egui::Button::new(if last { "Завершить экзамен" } else { "Далее ▶" })).clicked() {
                if last {
                    app.finish_reexam(date);
                } else {
                    rx.pos += 1;
                    rx.selected = None;
                    rx.answered = false;
                }
            }
        }
    });
}
