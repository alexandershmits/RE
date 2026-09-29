use eframe::egui::{self, RichText};

use super::{ACCENT, GOOD};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🧪 Placement: с какой недели вам стартовать?");
        ui.label(RichText::new("20 вопросов по базам. Не угадывайте — цель найти правильную ТОЧКУ ВХОДА, а не набить балл.").weak());
        ui.separator();

        let total = app.curriculum.placement.len();
        if total == 0 { return; }

        // track answers in a local session stored in app.quiz slot? No — dedicated state
        if app.placement.is_none() {
            if ui.button("▶ Начать тест").clicked() {
                app.placement = Some(crate::state::PlacementState { pos: 0, score: 0, selected: None, answered: false });
            }
            return;
        }
        let Some(pl) = app.placement.as_mut() else { return; };
        if pl.pos >= total {
            let score = pl.score;
            let pct = score as f32 / total as f32;
            let rec = if pct < 0.35 { "Неделя 0 — с полного нуля. Всё впереди!" }
                else if pct < 0.7 { "Недели 1–2 — C с самого начала, дальше быстрее." }
                else if pct < 0.95 { "Недели 3–6 — можно пропустить C, но пройдите asm и PE честно." }
                else { "Недели 7–9 — база есть, стартуйте с инструментов. Но прогоните чек-пойнты недель 1–6 как самопроверку." };
            ui.heading(format!("Результат: {score}/{total}"));
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(RichText::new(format!("Рекомендация: {rec}")).size(16.0));
            });
            if ui.button("Сбросить").clicked() { app.placement = None; }
            return;
        }

        let q = app.curriculum.placement[pl.pos].clone();
        ui.label(format!("Вопрос {}/{}", pl.pos + 1, total));
        ui.strong(RichText::new(&q.q).size(16.0));
        ui.add_space(6.0);
        let mut click: Option<usize> = None;
        for (i, a) in q.a.iter().enumerate() {
            let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
            if pl.answered {
                if i == q.correct { text = text.color(GOOD).strong(); }
                else if Some(i) == pl.selected { text = text.color(ACCENT).strong(); }
            }
            if ui.add_enabled(!pl.answered, egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap)).clicked() {
                click = Some(i);
            }
        }
        if let Some(sel) = click {
            pl.selected = Some(sel);
            pl.answered = true;
            if sel == q.correct { pl.score += 1; }
        }
        if pl.answered {
            let ok = pl.selected == Some(q.correct);
            ui.label(if ok { RichText::new("✔ Верно").color(GOOD) } else {
                RichText::new(format!("✘ Неверно. Тема относится к неделе {}", q.week)).color(ACCENT)
            });
            if ui.button("Далее →").clicked() {
                pl.pos += 1; pl.selected = None; pl.answered = false;
            }
        }
    });
}
