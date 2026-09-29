use super::now_and_day;
use super::theme::{good, soft, warn};
use super::widgets::tinted;
use crate::state::{AppState, CardSession, CARD_INTERVAL_DAYS, MISTAKE_GRADUATION_LEVEL};
use eframe::egui::{self, RichText};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🃏 Карточки (интервальное повторение)");
        let steps: Vec<String> = CARD_INTERVAL_DAYS[1..].iter().map(u64::to_string).collect();
        ui.label(
            RichText::new(format!(
                "«Знаю» отодвигает карточку на {} дн., «Повторить» возвращает её сегодня. \
                 Неверные ответы в квизах попадают сюда сами и уходят после {MISTAKE_GRADUATION_LEVEL} успешных повторений.",
                steps.join(" → ")
            ))
            .weak(),
        );
        ui.separator();
        if app.cards.is_some() {
            session(app, ui);
        } else {
            menu(app, ui);
        }
    });
}

fn menu(app: &mut AppState, ui: &mut egui::Ui) {
    let (_, today) = now_and_day();
    let deck = app.deck();
    if deck.is_empty() {
        ui.label("Колода пуста.");
        return;
    }
    let due = deck
        .iter()
        .filter(|c| app.card_due_day(&c.id) <= today)
        .count();
    let mistakes = deck.iter().filter(|c| c.mistake).count();
    ui.label(format!(
        "Колода: {} карточек, из них ошибок из квизов: {mistakes}",
        deck.len()
    ));
    let mut levels = [0usize; 6];
    for c in &deck {
        levels[usize::from(app.card_level(&c.id)).min(5)] += 1;
    }
    ui.horizontal(|ui| {
        for (level, n) in levels.iter().enumerate() {
            ui.label(format!("L{level}: {n}"));
        }
    });
    ui.add_space(8.0);
    if due > 0 {
        ui.label(
            RichText::new(format!("К повторению сегодня: {due}"))
                .color(warn())
                .strong(),
        );
        if ui
            .button("▶ Начать сессию (ошибки и слабые — первыми)")
            .clicked()
        {
            app.start_cards(true, today);
        }
    } else {
        ui.label(
            RichText::new("На сегодня всё повторено ✔")
                .color(good())
                .strong(),
        );
        if ui.button("Повторить всё равно").clicked() {
            app.start_cards(false, today);
        }
    }
}

fn session(app: &mut AppState, ui: &mut egui::Ui) {
    let Some(mut cs) = app.cards.take() else {
        return;
    };
    let (_, today) = now_and_day();
    let mut close = false;

    if cs.is_finished() {
        ui.heading("Сессия завершена!");
        ui.label(format!(
            "Знаю: {} · Повторить: {}",
            cs.known.len(),
            cs.again.len()
        ));
        ui.horizontal(|ui| {
            if !cs.again.is_empty()
                && ui
                    .button(format!(
                        "🔁 Ещё раз то, что не вспомнил ({})",
                        cs.again.len()
                    ))
                    .clicked()
            {
                cs = CardSession::new(std::mem::take(&mut cs.again));
            }
            close = ui.button("Закончить").clicked();
        });
    } else if let Some(card) = cs.current().cloned() {
        ui.label(RichText::new(format!("{}/{}", cs.pos + 1, cs.cards.len())).weak());
        ui.add(egui::ProgressBar::new(cs.pos as f32 / cs.cards.len() as f32).desired_height(6.0));
        ui.add_space(8.0);
        let fill = if card.mistake {
            soft(46, 38, 20)
        } else {
            ui.visuals().faint_bg_color
        };
        tinted(ui, fill, |ui| {
            if card.mistake {
                ui.label(
                    RichText::new("⚠ Ты ошибся в этом вопросе")
                        .color(warn())
                        .size(11.0),
                );
            }
            ui.strong(RichText::new(&card.front).size(17.0));
            if cs.show_back {
                ui.separator();
                ui.label(RichText::new(&card.back).size(15.0));
                ui.label(
                    RichText::new(format!("теги: {}", card.tags.join(", ")))
                        .weak()
                        .size(10.0),
                );
            }
        });
        ui.add_space(8.0);
        if !cs.show_back {
            if ui.button("Показать ответ").clicked() {
                cs.show_back = true;
            }
        } else {
            ui.horizontal(|ui| {
                let again = ui.button("🔁 Повторить").clicked();
                let known = ui.button("✅ Знаю").clicked();
                if again || known {
                    app.review_card(&card.id, known, today);
                    if known {
                        cs.known.push(card.id.clone());
                    } else {
                        cs.again.push(card.clone());
                    }
                    cs.pos += 1;
                    cs.show_back = false;
                }
            });
        }
    }

    if !close {
        app.cards = Some(cs);
    }
}
