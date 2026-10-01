//! Повторяющиеся элементы интерфейса.

use super::theme::{accent, code_bg, good};
use eframe::egui::{self, Color32, RichText};

/// Варианты ответа A), B), … Возвращает индекс нажатого варианта.
/// При `revealed` верный вариант подсвечивается зелёным, ошибочный выбор — красным.
pub fn choice_list(
    ui: &mut egui::Ui,
    answers: &[String],
    correct: usize,
    chosen: Option<usize>,
    revealed: bool,
) -> Option<usize> {
    let mut clicked = None;
    for (i, answer) in answers.iter().enumerate() {
        let mut text = RichText::new(format!("{}) {answer}", char::from(b'A' + i as u8)));
        if revealed {
            if i == correct {
                text = text.color(good()).strong();
            } else if chosen == Some(i) {
                text = text.color(accent()).strong();
            } else {
                // отключённая кнопка рисуется полупрозрачной (контраст < 3), а эти варианты читают ради разбора
                text = text.color(ui.visuals().weak_text_color());
            }
        }
        let mut button = egui::Button::new(text)
            .wrap_mode(egui::TextWrapMode::Wrap)
            .min_size(egui::vec2(0.0, 30.0));
        if revealed {
            // Не «отключаем» кнопку: отключённый виджет egui рисует вполовину прозрачным, и правильный ответ
            // выглядел бы блёклым. Без реакции на мышь клик просто невозможен.
            button = button.sense(egui::Sense::hover());
        }
        if ui.add(button).clicked() {
            clicked = Some(i);
        }
    }
    clicked
}

/// Полоса прогресса с подписью справа от неё. Подпись внутри полосы egui красит белым (цветом выделения),
/// а пустая полоса в светлой теме светло-серая: «0/12» и «7% курса» там не читались.
pub fn labeled_progress(
    ui: &mut egui::Ui,
    fraction: f32,
    width: f32,
    label: impl Into<egui::WidgetText>,
) {
    ui.horizontal(|ui| {
        ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).desired_width(width));
        ui.label(label);
    });
}

/// Рамка с заливкой на всю доступную ширину.
pub fn tinted(ui: &mut egui::Ui, fill: Color32, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).fill(fill).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add_contents(ui);
    });
}

/// Блок моноширинного кода/дизассемблера.
pub fn code_block<'a>(ui: &mut egui::Ui, lines: impl IntoIterator<Item = &'a str>) {
    tinted(ui, code_bg(), |ui| {
        for line in lines {
            ui.monospace(line);
        }
    });
}
