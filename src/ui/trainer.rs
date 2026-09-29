use super::theme::{accent, good};
use super::widgets::choice_list;
use crate::state::AppState;
use eframe::egui::{self, RichText};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        if app.quiz.is_some() {
            session(app, ui, &ctx);
        } else {
            menu(app, ui, &ctx);
        }
    });
}

fn menu(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    let cur = app.curriculum.clone();
    ui.heading("🎯 Тренажёр-квиз");
    ui.label("Проверь себя: понимание или имитация? Выбери модуль и начни сессию.");
    ui.add_space(8.0);
    if ui
        .button(format!("▶ Все модули ({})", cur.quizzes.len()))
        .clicked()
    {
        app.start_quiz(None);
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for m in &cur.modules {
            let n = cur.quizzes.iter().filter(|q| q.module == m.id).count();
            if n > 0
                && ui.button(format!("{} ({n})", m.name)).clicked()
                && !app.start_quiz(Some(m.id))
            {
                app.toast("В этом модуле пока нет вопросов", ctx);
            }
        }
    });
    ui.add_space(16.0);
    ui.separator();
    ui.label(RichText::new("Прогресс по вопросам:").weak());
    for m in &cur.modules {
        let quizzes: Vec<_> = cur.quizzes.iter().filter(|q| q.module == m.id).collect();
        if quizzes.is_empty() {
            continue;
        }
        let ok = quizzes
            .iter()
            .filter(|q| app.progress.quiz_correct.contains(&q.id))
            .count();
        ui.horizontal(|ui| {
            ui.label(format!("{} {}:", m.icon, m.name));
            ui.add(
                egui::ProgressBar::new(ok as f32 / quizzes.len() as f32)
                    .text(format!("{ok}/{}", quizzes.len()))
                    .desired_width(260.0),
            );
        });
    }
}

fn session(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(mut q) = app.quiz.take() else { return };
    let cur = app.curriculum.clone();
    let Some(quiz) = q.current().and_then(|i| cur.quizzes.get(i)) else {
        return;
    };

    let mut abort = false;
    ui.horizontal(|ui| {
        ui.heading(format!("Вопрос {}/{}", q.pos + 1, q.order.len()));
        if let Some(m) = q.filter_module {
            ui.label(RichText::new(cur.module_name(m)).weak());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            abort = ui.button("✖ Завершить сессию").clicked();
        });
    });
    ui.add(egui::ProgressBar::new(q.pos as f32 / q.order.len() as f32).desired_height(8.0));
    ui.add_space(12.0);
    ui.strong(RichText::new(&quiz.question).size(17.0));
    ui.add_space(6.0);

    if let Some(chosen) = choice_list(ui, &quiz.answers, quiz.correct, q.selected, q.submitted) {
        let correct = app.submit_quiz_answer(quiz, chosen);
        q.record(chosen, correct);
    }

    if q.submitted {
        ui.add_space(8.0);
        if q.selected == Some(quiz.correct) {
            ui.label(RichText::new("✔ Верно!").color(good()).size(16.0).strong());
        } else {
            let letter = char::from(b'A' + quiz.correct as u8);
            ui.label(
                RichText::new(format!("✘ Неверно. Правильный ответ: {letter}"))
                    .color(accent())
                    .size(16.0)
                    .strong(),
            );
            ui.label(RichText::new("💡 Вопрос добавлен в «Карточки»: он вернётся к повторению сегодня, затем через 1, 3 и 7 дней.").weak().size(11.0));
        }
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(RichText::new(format!("💡 {}", quiz.explain)));
        });
        ui.add_space(8.0);
        if ui.button("Далее →").clicked() {
            q.advance();
        }
    }

    if abort {
        q.abort();
    }
    if q.finished {
        let pct = q.correct_count * 100 / q.answered.max(1);
        app.toast(
            format!(
                "Сессия завершена: {}/{} ({pct}%)",
                q.correct_count, q.answered
            ),
            ctx,
        );
    } else {
        app.quiz = Some(q);
    }
}
