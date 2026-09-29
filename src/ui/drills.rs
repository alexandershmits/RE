use super::theme::{accent, good};
use super::widgets::{choice_list, code_block};
use crate::state::{xp, AppState, DrillState};
use eframe::egui::{self, RichText, ScrollArea};

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let cur = app.curriculum.clone();
    let sets = [
        ("🧮 Асм", "asm", cur.drills.asm.len()),
        ("📐 Адреса", "addr", cur.drills.addr.len()),
        ("🧩 Паттерны", "pat", cur.drills.pattern.len()),
        ("🐍 Скрипты", "scr", cur.drills.script.len()),
    ];
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🔁 Дриллы: автоматизм через повторение");
        ui.label(RichText::new("Короткие задачи на 30 секунд – 3 минуты. XP за каждый дрилл — один раз. Адресный дрилл — арифметика RVA/VA/file offset.").weak());
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for (i, (name, prefix, total)) in sets.iter().enumerate() {
                let label = format!("{name} ({}/{total})", app.drills_solved_in(prefix));
                if ui.selectable_label(app.drill.which == i, label).clicked() {
                    app.drill = DrillState { which: i, ..DrillState::default() };
                }
            }
        });
        ui.add_space(8.0);
        ScrollArea::vertical().id_salt("drills").show(ui, |ui| match app.drill.which {
            0 => choice_drill(app, ui, "asm", cur.drills.asm.iter().map(Choice::asm).collect()),
            1 => choice_drill(app, ui, "addr", cur.drills.addr.iter().map(Choice::addr).collect()),
            2 => choice_drill(app, ui, "pat", cur.drills.pattern.iter().map(Choice::pattern).collect()),
            _ => script_drill(app, ui),
        });
    });
}

/// Дрилл с выбором ответа: условие (заголовок и необязательный код) + варианты.
struct Choice<'a> {
    title: &'a str,
    code: Vec<&'a str>,
    question: &'a str,
    answers: &'a [String],
    correct: usize,
    explain: &'a str,
}

impl<'a> Choice<'a> {
    fn asm(t: &'a crate::curriculum::AsmDrill) -> Self {
        Choice {
            title: &t.c,
            code: t.asm.iter().map(String::as_str).collect(),
            question: "Что вернёт функция?",
            answers: &t.answers,
            correct: t.correct,
            explain: &t.explain,
        }
    }
    fn addr(t: &'a crate::curriculum::AddrDrill) -> Self {
        Choice {
            title: &t.q,
            code: Vec::new(),
            question: "",
            answers: &t.answers,
            correct: t.correct,
            explain: &t.explain,
        }
    }
    fn pattern(t: &'a crate::curriculum::PatternDrill) -> Self {
        Choice {
            title: "",
            code: t.asm.lines().collect(),
            question: "Какой это паттерн?",
            answers: &t.answers,
            correct: t.correct,
            explain: &t.explain,
        }
    }
}

fn choice_drill(app: &mut AppState, ui: &mut egui::Ui, prefix: &str, tasks: Vec<Choice>) {
    if tasks.is_empty() {
        return;
    }
    let idx = app.drill.idx.min(tasks.len() - 1);
    let t = &tasks[idx];
    if !t.title.is_empty() {
        ui.strong(RichText::new(t.title).size(15.0));
        ui.add_space(4.0);
    }
    if !t.code.is_empty() {
        code_block(ui, t.code.iter().copied());
        ui.add_space(4.0);
    }
    if !t.question.is_empty() {
        ui.label(t.question);
    }
    if let Some(chosen) = choice_list(
        ui,
        t.answers,
        t.correct,
        app.drill.choice,
        app.drill.checked,
    ) {
        app.drill.choice = Some(chosen);
        app.drill.checked = true;
        if chosen == t.correct && app.solve_drill(&format!("{prefix}{idx}"), xp::DRILL) {
            let ctx = ui.ctx().clone();
            app.toast(format!("Верно! +{} XP", xp::DRILL), &ctx);
        }
    }
    if app.drill.checked {
        if app.drill.choice == Some(t.correct) {
            ui.label(RichText::new("✔ Верно!").color(good()).strong());
        } else {
            ui.label(RichText::new("✖ Неверно.").color(accent()).strong());
        }
    }
    if app.drill.show_answer {
        ui.label(RichText::new(format!("💡 {}", t.explain)).weak());
    }
    ui.horizontal(|ui| {
        if ui.button("➡ Следующая").clicked() {
            app.drill = DrillState {
                which: app.drill.which,
                idx: (idx + 1) % tasks.len(),
                ..DrillState::default()
            };
        }
        if ui.button("💡 Ответ").clicked() {
            app.drill.show_answer = !app.drill.show_answer;
        }
        ui.label(
            RichText::new(format!("{}/{}", idx + 1, tasks.len()))
                .weak()
                .size(11.0),
        );
    });
}

fn script_drill(app: &mut AppState, ui: &mut egui::Ui) {
    let cur = app.curriculum.clone();
    let tasks = &cur.drills.script;
    if tasks.is_empty() {
        return;
    }
    let idx = app.drill.idx.min(tasks.len() - 1);
    let t = &tasks[idx];
    ui.strong(RichText::new(&t.task).size(15.0));
    ui.add_space(4.0);
    ui.label(
        RichText::new("Напишите решение в уме или в редакторе, затем откройте эталон.").weak(),
    );
    let (mut peek, mut known, mut skip) = (false, false, false);
    ui.horizontal(|ui| {
        peek = ui.button("💡 Показать эталон").clicked();
        known = ui.button("✔ Знаю это").clicked();
        skip = ui.button("➡ Пропустить").clicked();
    });
    if peek {
        app.drill.show_answer = !app.drill.show_answer;
    }
    if app.drill.show_answer {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.monospace(&t.answer);
            ui.label(RichText::new(format!("💬 {}", t.hint)).weak());
        });
    }
    if known && app.solve_drill(&format!("scr{idx}"), xp::DRILL) {
        let ctx = ui.ctx().clone();
        app.toast(format!("Засчитано! +{} XP", xp::DRILL), &ctx);
    }
    if known || skip {
        app.drill = DrillState {
            which: app.drill.which,
            idx: (idx + 1) % tasks.len(),
            ..DrillState::default()
        };
    }
    ui.label(
        RichText::new(format!("{}/{}", idx + 1, tasks.len()))
            .weak()
            .size(11.0),
    );
}
