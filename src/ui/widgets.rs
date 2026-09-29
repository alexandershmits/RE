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
            }
        }
        let button = egui::Button::new(text)
            .wrap_mode(egui::TextWrapMode::Wrap)
            .min_size(egui::vec2(0.0, 30.0));
        if ui.add_enabled(!revealed, button).clicked() {
            clicked = Some(i);
        }
    }
    clicked
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
