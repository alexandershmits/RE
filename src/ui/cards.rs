use eframe::egui::{self, RichText};

use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🃏 Карточки (spaced repetition)");
        ui.label(RichText::new("Уровень карточки растёт при ответе «знаю» и падает при «повторить». Интервалы: 1д → 3д → 7д → 21д → 60д.").weak());
        ui.separator();

        let total = app.curriculum.flashcards.len();
        if app.cards.is_none() {
            if total == 0 { return; }
            ui.label(format!("Колода: {total} карточек"));
            // box stats
            let mut levels = [0usize; 6];
            for c in &app.curriculum.flashcards {
                let l = app.progress.card_levels.get(&c.id).copied().unwrap_or(0) as usize;
                levels[l.min(5)] += 1;
            }
            ui.horizontal(|ui| {
                for (i, n) in levels.iter().enumerate() {
                    ui.label(format!("L{i}: {n}"));
                }
            });
            if ui.button("▶ Начать сессию (слабые — первыми)").clicked() {
                // order: lowest level first, shuffled within level
                let mut idx: Vec<usize> = (0..total).collect();
                idx.sort_by_key(|&i| app.progress.card_levels.get(&app.curriculum.flashcards[i].id).copied().unwrap_or(0));
                app.cards = Some(crate::state::CardSession::new(0)); // placeholder then override
                if let Some(cs) = app.cards.as_mut() {
                    cs.order = idx;
                    cs.pos = 0;
                }
            }
            return;
        }

        let Some(cs) = app.cards.as_mut() else { return; };
        if cs.pos >= cs.order.len() {
            // session end
            let known = cs.known.len();
            let again = cs.again.len();
            ui.heading("Сессия завершена!");
            ui.label(format!("Знаю: {known} · Повторить: {again}"));
            if ui.button("Закончить").clicked() { app.cards = None; }
            app.save();
            return;
        }
        let card = app.curriculum.flashcards[cs.order[cs.pos]].clone();
        ui.label(RichText::new(format!("{}/{}", cs.pos + 1, cs.order.len())).weak());
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 16.0);
            ui.strong(RichText::new(&card.front).size(17.0));
            if cs.show_back {
                ui.separator();
                ui.label(RichText::new(&card.back).size(15.0));
                ui.label(RichText::new(format!("теги: {}", card.tags.join(", "))).weak().size(10.0));
            }
        });
        ui.add_space(8.0);
        if !cs.show_back {
            if ui.button("Показать ответ").clicked() { cs.show_back = true; }
        } else {
            ui.horizontal(|ui| {
                if ui.button("🔁 Повторить").clicked() {
                    cs.again.push(cs.order[cs.pos]);
                    let lvl = app.progress.card_levels.get(&card.id).copied().unwrap_or(0).saturating_sub(1);
                    app.progress.card_levels.insert(card.id.clone(), lvl);
                    cs.pos += 1; cs.show_back = false;
                }
                if ui.button("✅ Знаю").clicked() {
                    cs.known.push(cs.order[cs.pos]);
                    let lvl = (app.progress.card_levels.get(&card.id).copied().unwrap_or(0) + 1).min(5);
                    app.progress.card_levels.insert(card.id.clone(), lvl);
                    cs.pos += 1; cs.show_back = false;
                }
            });
        }
    });
}
