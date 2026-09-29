use eframe::egui::{self, RichText};

use super::{ACCENT, GOOD, WARN};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    use crate::opponent;
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(RichText::new("🥋 Socratic-оппонент").color(ACCENT));
        ui.label("Атакует твои объяснения: не даёт ответов — задает каверзные вопросы. Пересказ и «так принято» здесь не работают: только понимание. Лучший суррогат ментора в solo-обучении.");
        ui.add_space(10.0);

        // Режим LLM
        let llm_online = opponent::ollama_available();
        ui.horizontal(|ui| {
            if llm_online {
                ui.label(RichText::new("🟢 Ollama обнаружен — доступен живой LLM-режим").color(GOOD));
                if app.opponent.llm_model.is_empty() {
                    app.opponent.llm_model = "llama3.1".into();
                }
                ui.add(egui::TextEdit::singleline(&mut app.opponent.llm_model).desired_width(120.0));
            } else {
                ui.label(RichText::new("⚪ Ollama не запущен — работает офлайн-банк из 14 каверзных вопросов. Для живого режима: ollama.com → ollama pull llama3.1 → ollama serve").weak());
            }
        });
        ui.add_space(8.0);
        ui.separator();

        let n = opponent::QUESTIONS.len();
        let qs = &opponent::QUESTIONS[app.opponent.q_index % n];

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("[{}] ", qs.topic)).color(WARN).strong());
            ui.label(RichText::new(format!("вопрос {} из {}", app.opponent.q_index % n + 1, n)).weak());
        });
        ui.add_space(4.0);
        ui.label(RichText::new(qs.question).size(16.0));
        ui.add_space(8.0);

        let changed = egui::TextEdit::multiline(&mut app.opponent.answer)
            .desired_rows(5)
            .hint_text("Опиши механизм своими словами. Оппонент будет искать слабые места...")
            .show(ui)
            .response
            .changed();
        let _ = changed;

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("⚔ Отправить оппоненту").color(ACCENT))).clicked() {
                let v = opponent::evaluate(qs, &app.opponent.answer);
                let best = app.opponent.best_scores.entry(qs.topic.to_string()).or_insert(0);
                if v.score > *best { *best = v.score; }
                app.opponent.verdict = Some((v.score, v.critique));
                if llm_online && !app.opponent.answer.trim().is_empty() {
                    app.opponent.llm_in_flight = true;
                }
            }
            if ui.button("🎲 Другой вопрос").clicked() {
                app.opponent.q_index = (app.opponent.q_index + 1) % n;
                app.opponent.answer.clear();
                app.opponent.verdict = None;
                app.opponent.llm_reply = None;
            }
        });

        if let Some((score, critique)) = &app.opponent.verdict {
            ui.add_space(10.0);
            let color = if *score >= 70 { GOOD } else if *score >= 40 { WARN } else { ACCENT };
            egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(38, 30, 40)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Оценка: {score}/100")).color(color).strong());
                    ui.add(egui::ProgressBar::new(*score as f32 / 100.0).desired_width(200.0));
                });
                ui.label(critique);
            });
        }

        // LLM follow-up
        if app.opponent.llm_in_flight {
            ui.add_space(6.0);
            ui.label(RichText::new("🤖 Живой оппонент думает...").weak());
            let topic = qs.topic.to_string();
            let answer = app.opponent.answer.clone();
            let model = app.opponent.llm_model.clone();
            // блокирующий вызов в фоне отложим: для простоты выполняем синхронно по кнопке
            app.opponent.llm_in_flight = false;
            match opponent::ollama_ask(&model, &topic, &answer) {
                Ok(reply) => {
                    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(30, 34, 46)).show(ui, |ui| {
                        ui.label(RichText::new("🤖 Ollama:").strong());
                        ui.label(&reply);
                    });
                    app.opponent.llm_reply = Some(reply);
                }
                Err(e) => {
                    app.opponent.llm_reply = Some(format!("(LLM недоступен: {e})"));
                }
            }
        } else if let Some(r) = &app.opponent.llm_reply {
            egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(30, 34, 46)).show(ui, |ui| {
                ui.label(RichText::new("🤖 Ollama:").strong());
                ui.label(r);
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("📊 Лучшие баллы по темам:").strong());
        let mut rows: Vec<_> = app.opponent.best_scores.iter().collect();
        rows.sort_by_key(|(_, s)| std::cmp::Reverse(**s));
        for (topic, score) in rows {
            ui.horizontal(|ui| {
                ui.label(topic.as_str());
                ui.add(egui::ProgressBar::new(*score as f32 / 100.0).desired_width(160.0).text(format!("{score}")));
            });
        }
        if app.opponent.best_scores.is_empty() {
            ui.label(RichText::new("Пока пусто — ответь хотя бы на один вопрос.").weak());
        }
    });
}
