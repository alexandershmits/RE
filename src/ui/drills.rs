use eframe::egui::{self, RichText, ScrollArea};

use super::{ACCENT, GOOD};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🔁 Дриллы: автоматизм через повторение");
        ui.label(RichText::new("Короткие задачи 30 сек – 3 мин. Адресный дрилл генерирует числа — практикуйтесь бесконечно.").weak());
        ui.separator();
        ui.horizontal(|ui| {
            for (i, name) in ["🧮 Асм (20)", "📐 Адреса (24)", "🧩 Паттерны (15)", "🐍 Скрипты (10)"].iter().enumerate() {
                if ui.selectable_label(app.drill.which == i, *name).clicked() {
                    app.drill = Default::default();
                    app.drill.which = i;
                }
            }
        });
        ui.add_space(8.0);
        ScrollArea::vertical().id_salt("drills").show(ui, |ui| {
            match app.drill.which {
                0 => drill_asm(app, ui),
                1 => drill_addr(app, ui),
                2 => drill_pattern(app, ui),
                _ => drill_script(app, ui),
            }
        });
    });
}

fn drill_nav(app: &mut AppState, ui: &mut egui::Ui, total: usize, xp: u32) {
    ui.horizontal(|ui| {
        if ui.button("➡ Следующая").clicked() {
            app.drill.idx = (app.drill.idx + 1) % total;
            app.drill.choice = None;
            app.drill.checked = false;
            app.drill.show_answer = false;
        }
        if ui.button("💡 Ответ").clicked() {
            app.drill.show_answer = !app.drill.show_answer;
        }
        ui.label(
            RichText::new(format!("решено в сессии: {}", app.drill.solved.len()))
                .weak()
                .size(11.0),
        );
    });
    let _ = xp;
}

fn drill_common_choice_ui(
    app: &mut AppState,
    ui: &mut egui::Ui,
    answers: &[String],
    correct: usize,
    id_tag: &str,
    xp: u32,
) {
    let mut click: Option<usize> = None;
    for (i, a) in answers.iter().enumerate() {
        let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
        if app.drill.checked {
            if i == correct {
                text = text.color(GOOD).strong();
            } else if app.drill.choice == Some(i) {
                text = text.color(ACCENT).strong();
            }
        }
        if ui
            .add_enabled(
                !app.drill.checked,
                egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap),
            )
            .clicked()
        {
            click = Some(i);
        }
    }
    if let Some(c) = click {
        app.drill.choice = Some(c);
        app.drill.checked = true;
        let ok = c == correct;
        if ok {
            let key = format!("{id_tag}{}", app.drill.idx);
            app.drill.solved.insert(key.clone());
            app.progress.drills_solved.insert(key);
            app.add_xp(xp);
            app.toast(format!("Верно! +{xp} XP"), ui.ctx());
        }
    }
    if app.drill.checked {
        let ok = app.drill.choice == Some(correct);
        if ok {
            ui.label(RichText::new("✔ Верно!").color(GOOD).strong());
        } else {
            ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong());
        }
    }
}

fn drill_asm(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = app.curriculum.drills.asm.clone();
    if tasks.is_empty() {
        return;
    }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    ui.strong(RichText::new(&t.c).size(15.0));
    ui.add_space(4.0);
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(12, 13, 18))
        .show(ui, |ui| {
            for line in &t.asm {
                ui.monospace(line);
            }
        });
    ui.add_space(4.0);
    ui.label("Что вернёт функция?");
    let correct = t.correct;
    let answers = t.answers.clone();
    drill_common_choice_ui(app, ui, &answers, correct, "asm", 10);
    if app.drill.show_answer {
        ui.label(RichText::new(format!("💡 {}", t.explain)).weak());
    }
    drill_nav(app, ui, tasks.len(), 10);
}

fn drill_addr(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = app.curriculum.drills.addr.clone();
    if tasks.is_empty() {
        return;
    }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    ui.strong(RichText::new(&t.q).size(15.0));
    ui.add_space(4.0);
    let correct = t.correct;
    let answers = t.answers.clone();
    drill_common_choice_ui(app, ui, &answers, correct, "addr", 10);
    if app.drill.show_answer {
        ui.label(RichText::new(format!("💡 {}", t.explain)).weak());
    }
    drill_nav(app, ui, tasks.len(), 10);
}

fn drill_pattern(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = app.curriculum.drills.pattern.clone();
    if tasks.is_empty() {
        return;
    }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(12, 13, 18))
        .show(ui, |ui| {
            for line in t.asm.lines() {
                ui.monospace(line);
            }
        });
    ui.add_space(4.0);
    ui.label("Какой это паттерн?");
    let correct = t.correct;
    let answers = t.answers.clone();
    drill_common_choice_ui(app, ui, &answers, correct, "pat", 10);
    if app.drill.show_answer {
        ui.label(RichText::new(format!("💡 {}", t.explain)).weak());
    }
    drill_nav(app, ui, tasks.len(), 10);
}

fn drill_script(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = app.curriculum.drills.script.clone();
    if tasks.is_empty() {
        return;
    }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    ui.strong(RichText::new(&t.task).size(15.0));
    ui.add_space(4.0);
    ui.label(RichText::new("Напишите решение в уме/в редакторе, затем откройте эталон.").weak());
    ui.horizontal(|ui| {
        if ui.button("💡 Показать эталон").clicked() {
            app.drill.show_answer = !app.drill.show_answer;
        }
        if ui.button("✔ Знаю это").clicked() {
            let id = format!("scr{}", app.drill.idx);
            if app.drill.solved.insert(id.clone()) {
                app.progress.drills_solved.insert(id);
                app.add_xp(10);
                app.toast("Засчитано! +10 XP", ui.ctx());
            }
            app.drill.idx = (app.drill.idx + 1) % tasks.len();
            app.drill.show_answer = false;
            app.save();
        }
        if ui.button("➡ Пропустить").clicked() {
            app.drill.idx = (app.drill.idx + 1) % tasks.len();
            app.drill.show_answer = false;
        }
    });
    if app.drill.show_answer {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.monospace(&t.answer);
            ui.label(RichText::new(format!("💬 {}", t.hint)).weak());
        });
    }
    ui.label(
        RichText::new(format!("решено в сессии: {}", app.drill.solved.len()))
            .weak()
            .size(11.0),
    );
}
