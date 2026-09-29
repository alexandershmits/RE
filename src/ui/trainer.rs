use eframe::egui::{self, RichText};

use super::{ACCENT, GOOD};
use crate::state::{AppState, QuizSession};

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.quiz.is_none() {
            ui.heading("🎯 Тренажёр-квиз");
            ui.label("Проверь себя: понимание или имитация? Выбери модуль и начни сессию.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("▶ Все модули").clicked() {
                    app.quiz = Some(QuizSession::new(app.curriculum.quizzes.len(), None));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for m in &app.curriculum.modules {
                    let n = app
                        .curriculum
                        .quizzes
                        .iter()
                        .filter(|q| q.module == m.id)
                        .count();
                    if n > 0 && ui.button(format!("{} ({n})", m.name)).clicked() {
                        app.quiz = Some(QuizSession::new(app.curriculum.quizzes.len(), Some(m.id)));
                    }
                }
            });
            ui.add_space(16.0);
            ui.separator();
            ui.label(RichText::new("Прогресс по вопросам:").weak());
            for m in &app.curriculum.modules {
                let total = app.curriculum.quizzes.iter().filter(|q| q.module == m.id).count();
                let ok = app
                    .curriculum
                    .quizzes
                    .iter()
                    .filter(|q| q.module == m.id)
                    .filter(|q| app.progress.quiz_correct.contains(&q.id))
                    .count();
                if total > 0 {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}:", m.icon, m.name));
                        ui.add(
                            egui::ProgressBar::new(ok as f32 / total as f32)
                                .text(format!("{ok}/{total}"))
                                .desired_width(260.0),
                        );
                    });
                }
            }
            return;
        }

        // take quiz session out to avoid borrow conflicts
        let mut q = app.quiz.take().unwrap();
        let pos = q.pos.min(q.order.len() - 1);
        let quiz_idx = q.order[pos];
        let quiz = app.curriculum.quizzes[quiz_idx].clone();

        let mut finish = false;
        {
            ui.horizontal(|ui| {
                ui.heading(format!("Вопрос {}/{}", q.pos + 1, q.order.len()));
                if let Some(m) = q.filter_module {
                    ui.label(RichText::new(app.curriculum.module_name(m)).weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Завершить сессию").clicked() {
                        finish = false;
                        q.pos = q.order.len(); // mark aborted
                    }
                });
            });
            ui.add(
                egui::ProgressBar::new((q.pos as f32) / q.order.len() as f32)
                    .desired_height(8.0),
            );
            ui.add_space(12.0);

            ui.strong(RichText::new(&quiz.question).size(17.0));
            ui.add_space(6.0);

            let mut click_answer: Option<usize> = None;
            for (i, a) in quiz.answers.iter().enumerate() {
                let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
                if q.submitted {
                    if i == quiz.correct {
                        text = text.color(GOOD).strong();
                    } else if Some(i) == q.selected {
                        text = text.color(ACCENT).strong();
                    }
                }
                let enabled = !q.submitted;
                let resp = ui.add_enabled(
                    enabled,
                    egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap).min_size(egui::vec2(0.0, 30.0)),
                );
                if resp.clicked() {
                    click_answer = Some(i);
                }
            }

            if let Some(sel) = click_answer {
                q.selected = Some(sel);
                q.submitted = true;
                q.answered += 1;
                if app.submit_quiz_answer(&quiz, sel) {
                    q.correct_count += 1;
                }
            }

            if q.submitted {
                let ok = q.selected == Some(quiz.correct);
                ui.add_space(8.0);
                if ok {
                    ui.label(RichText::new("✔ Верно!").color(GOOD).size(16.0).strong());
                } else {
                    ui.label(
                        RichText::new(format!(
                            "✘ Неверно. Правильный ответ: {}",
                            char::from(b'A' + quiz.correct as u8)
                        ))
                        .color(ACCENT)
                        .size(16.0)
                        .strong(),
                    );
                ui.label(RichText::new("💡 Тема уйдёт в карточки: «Повторить» понижает уровень карточки и поднимает её в очереди.").weak().size(11.0));
                }
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("💡 {}", quiz.explain)));
                });
                ui.add_space(8.0);
                if ui.button("Далее →").clicked() {
                    q.pos += 1;
                    q.selected = None;
                    q.submitted = false;
                    if q.pos >= q.order.len() {
                        q.finished = true;
                    }
                }
            }
        } // ui borrow ends

        app.save();
        if q.finished {
            let pct = (q.correct_count as f32 / q.answered.max(1) as f32 * 100.0) as u32;
            app.toast(format!("Сессия завершена: {}/{} ({pct}%)", q.correct_count, q.answered), ctx);
        } else {
            app.quiz = Some(q);
        }
    });
}
