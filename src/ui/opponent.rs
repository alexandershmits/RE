use super::theme::{accent, good, soft, warn};
use super::widgets::tinted;
use crate::jobs::{Job, JobState};
use crate::opponent;
use crate::state::AppState;
use eframe::egui::{self, RichText};
use std::time::{Duration, Instant};

const RECHECK_AFTER: Duration = Duration::from_secs(15);

/// Принимает результаты фоновых запросов (вызывается каждый кадр из любой вкладки).
pub(super) fn poll(app: &mut AppState, ctx: &egui::Context) {
    if let Some(state) = app.opponent.ollama_job.as_ref().map(|j| j.poll()) {
        match state {
            JobState::Running => ctx.request_repaint_after(Duration::from_millis(100)),
            JobState::Done(online) => {
                app.opponent.ollama_online = Some(online);
                app.opponent.ollama_job = None;
            }
            JobState::Failed => app.opponent.ollama_job = None,
        }
    }
    if let Some(state) = app.opponent.llm_job.as_ref().map(|j| j.poll()) {
        match state {
            JobState::Running => ctx.request_repaint_after(Duration::from_millis(200)),
            JobState::Done(result) => {
                app.opponent.llm_reply =
                    Some(result.unwrap_or_else(|e| format!("(LLM недоступен: {e})")));
                app.opponent.llm_job = None;
            }
            JobState::Failed => {
                app.opponent.llm_reply =
                    Some("(LLM недоступен: фоновый запрос завершился аварийно)".into());
                app.opponent.llm_job = None;
            }
        }
    }
}

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let stale = app
        .opponent
        .ollama_checked
        .is_none_or(|t| t.elapsed() > RECHECK_AFTER);
    if stale && app.opponent.ollama_job.is_none() {
        app.opponent.ollama_checked = Some(Instant::now());
        app.opponent.ollama_job = Some(Job::spawn(&ctx, opponent::ollama_available));
    }
    let online = app.opponent.ollama_online == Some(true);
    let questions = opponent::QUESTIONS;
    let qs = &questions[app.opponent.q_index % questions.len()];

    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading(RichText::new("🥋 Socratic-оппонент").color(accent()));
        ui.label("Атакует твои объяснения: не даёт ответов — задаёт каверзные вопросы. Пересказ и «так принято» здесь не работают: только понимание. Лучший суррогат ментора в solo-обучении.");
        ui.add_space(10.0);

        ui.horizontal_wrapped(|ui| {
            if online {
                ui.label(RichText::new("🟢 Ollama обнаружен — доступен живой LLM-режим (запросы идут только на localhost)").color(good()));
                if app.opponent.llm_model.is_empty() {
                    app.opponent.llm_model = "llama3.1".into();
                }
                ui.add(egui::TextEdit::singleline(&mut app.opponent.llm_model).desired_width(120.0));
            } else {
                ui.label(RichText::new(format!(
                    "⚪ Ollama не запущен — работает офлайн-банк из {} каверзных вопросов. Для живого режима: ollama.com → ollama pull llama3.1.",
                    questions.len()
                )).weak());
            }
        });
        ui.add_space(8.0);
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("[{}] ", qs.topic)).color(warn()).strong());
            ui.label(RichText::new(format!("вопрос {} из {}", app.opponent.q_index % questions.len() + 1, questions.len())).weak());
        });
        ui.add_space(4.0);
        ui.label(RichText::new(qs.question).size(16.0));
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::multiline(&mut app.opponent.answer)
                .desired_rows(5)
                .desired_width(f32::INFINITY)
                .hint_text("Опиши механизм своими словами. Оппонент будет искать слабые места…"),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("⚔ Отправить оппоненту").color(accent()))).clicked() {
                let verdict = opponent::evaluate(qs, &app.opponent.answer);
                app.record_opponent_score(qs.topic, verdict.score);
                app.opponent.verdict = Some((verdict.score, verdict.critique));
                app.opponent.llm_reply = None;
                app.opponent.llm_job = None; // ответ модели на прежний текст больше не нужен
                if online && !app.opponent.answer.trim().is_empty() {
                    let (model, topic, answer) =
                        (app.opponent.llm_model.clone(), qs.topic.to_string(), app.opponent.answer.clone());
                    app.opponent.llm_job = Some(Job::spawn(&ctx, move || opponent::ollama_ask(&model, &topic, &answer)));
                }
            }
            if ui.button("➡ Следующий вопрос").clicked() {
                app.opponent.q_index = (app.opponent.q_index + 1) % questions.len();
                app.opponent.llm_job = None; // ответ модели на прежний вопрос больше не нужен
                app.opponent.answer.clear();
                app.opponent.verdict = None;
                app.opponent.llm_reply = None;
            }
        });

        if let Some((score, critique)) = &app.opponent.verdict {
            ui.add_space(10.0);
            let color = if *score >= 70 { good() } else if *score >= 40 { warn() } else { accent() };
            tinted(ui, soft(38, 30, 40), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Оценка: {score}/100")).color(color).strong());
                    ui.add(egui::ProgressBar::new(f32::from(*score) / 100.0).desired_width(200.0));
                });
                ui.label(critique);
            });
        }

        if app.opponent.llm_job.is_some() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("🤖 Живой оппонент думает…").weak());
            });
        } else if let Some(reply) = &app.opponent.llm_reply {
            tinted(ui, soft(30, 34, 46), |ui| {
                ui.label(RichText::new("🤖 Ollama:").strong());
                ui.label(reply);
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("📊 Лучшие баллы по темам:").strong());
        let mut rows: Vec<_> = app.progress.opponent_best.iter().collect();
        rows.sort_by_key(|(_, score)| std::cmp::Reverse(**score));
        for (topic, score) in &rows {
            ui.horizontal(|ui| {
                ui.label(topic.as_str());
                ui.add(egui::ProgressBar::new(f32::from(**score) / 100.0).desired_width(160.0).text(score.to_string()));
            });
        }
        if rows.is_empty() {
            ui.label(RichText::new("Пока пусто — ответь хотя бы на один вопрос.").weak());
        }
    });
}
