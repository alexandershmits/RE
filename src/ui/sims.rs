use super::now_and_day;
use super::theme::{accent, good, warn};
use super::widgets::{choice_list, code_block};
use crate::rng::time_seed;
use crate::simulators::{check_gen, generate, parse_hex, GenKind, OepTask, PeTask, RegTask};
use crate::state::{xp, AppState, SimState};
use eframe::egui::{self, RichText, ScrollArea};

const MODES: [&str; 4] = [
    "🧮 Регистры и инструкции",
    "📦 PE-байты",
    "🔍 Поиск OEP",
    "🎲 Бесконечный генератор",
];

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("⚙ Микро-симуляторы");
        ui.label(RichText::new("Потрогайте теорию руками, не открывая отладчик: предскажи регистры, распарси PE, найди OEP. XP — один раз за задачу.").weak());
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            for (i, name) in MODES.iter().enumerate() {
                if ui.selectable_label(app.sim.which == i, *name).clicked() {
                    app.sim = SimState { which: i, ..SimState::default() };
                }
            }
        });
        ui.add_space(8.0);
        ScrollArea::vertical().id_salt("sims").show(ui, |ui| match app.sim.which {
            0 => registers(app, ui),
            1 => pe(app, ui),
            2 => oep(app, ui),
            _ => generative(app, ui),
        });
    });
}

fn next_task(app: &mut AppState, total: usize) {
    app.sim.task_idx = (app.sim.task_idx + 1) % total.max(1);
    app.sim.user_input.clear();
    app.sim.checked = false;
    app.sim.choice = None;
    app.sim.show_answer = false;
}

fn registers(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = RegTask::all();
    let idx = app.sim.task_idx % tasks.len();
    let task = &tasks[idx];

    ui.strong(RichText::new(&task.title).size(16.0));
    ui.label(
        RichText::new("Введите hex-значения (без 0x) для каждого регистра ПОСЛЕ выполнения кода:")
            .weak(),
    );
    ui.add_space(4.0);
    code_block(ui, task.code.iter().map(String::as_str));
    ui.add_space(6.0);

    egui::Grid::new("reginputs").num_columns(2).show(ui, |ui| {
        for (reg, initial) in &task.init {
            ui.label(format!("{reg} = 0x"));
            let input = app.sim.user_input.entry((*reg).to_string()).or_default();
            let edit = egui::TextEdit::singleline(input)
                .desired_width(140.0)
                .font(egui::TextStyle::Monospace);
            if ui.add(edit).changed() {
                app.sim.checked = false;
            }
            if *initial != 0 {
                ui.label(
                    RichText::new(format!("(в начале {initial:#x})"))
                        .weak()
                        .size(11.0),
                );
            }
            ui.end_row();
        }
    });
    let (mut check, mut skip, mut peek) = (false, false, false);
    ui.horizontal(|ui| {
        check = ui.button("✔ Проверить").clicked();
        skip = ui.button("➡ Следующая задача").clicked();
        peek = ui.button("💡 Ответ").clicked();
    });

    if check {
        let all_ok = task.answers.iter().all(|(reg, value)| {
            app.sim.user_input.get(*reg).and_then(|s| parse_hex(s)) == Some(*value)
        });
        app.sim.checked = true;
        app.sim.last_ok = all_ok;
        if all_ok {
            let paid = app.award_sim(&format!("reg{idx}"), xp::SIM_REGS);
            let ctx = ui.ctx().clone();
            app.toast(
                if paid {
                    format!("Верно! +{} XP", xp::SIM_REGS)
                } else {
                    "Верно! (XP за эту задачу уже получен)".into()
                },
                &ctx,
            );
        }
    }
    if app.sim.checked {
        if app.sim.last_ok {
            ui.label(RichText::new("✔ Всё верно!").color(good()).strong());
        } else {
            ui.label(
                RichText::new("✖ Есть ошибки — проверьте ещё раз или посмотрите ответ.")
                    .color(accent())
                    .strong(),
            );
        }
    }
    if peek {
        app.sim.show_answer = !app.sim.show_answer;
    }
    if app.sim.show_answer {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            for (reg, value) in &task.answers {
                ui.monospace(format!("{reg} = 0x{value:X}"));
            }
            ui.label(RichText::new(&task.explain).weak());
        });
    }
    if skip {
        next_task(app, tasks.len());
    }
}

/// Вопрос с вариантами ответа (PE и OEP).
struct ChoiceTask<'a> {
    key: String,
    reward: u32,
    answers: &'a [String],
    correct: usize,
    explain: &'a str,
    total: usize,
}

fn choice_task(app: &mut AppState, ui: &mut egui::Ui, task: ChoiceTask) {
    let ChoiceTask {
        key,
        reward,
        answers,
        correct,
        explain,
        total,
    } = task;
    if let Some(chosen) = choice_list(ui, answers, correct, app.sim.choice, app.sim.checked) {
        app.sim.choice = Some(chosen);
        app.sim.checked = true;
        app.sim.last_ok = chosen == correct;
        if app.sim.last_ok {
            let paid = app.award_sim(&key, reward);
            let ctx = ui.ctx().clone();
            app.toast(
                if paid {
                    format!("Верно! +{reward} XP")
                } else {
                    "Верно! (XP за эту задачу уже получен)".into()
                },
                &ctx,
            );
        }
    }
    if app.sim.checked {
        if app.sim.last_ok {
            ui.label(RichText::new("✔ Верно!").color(good()).strong());
        } else {
            ui.label(RichText::new("✖ Неверно.").color(accent()).strong());
        }
        ui.label(RichText::new(format!("💡 {explain}")).weak());
    }
    if ui.button("➡ Следующая").clicked() {
        next_task(app, total);
    }
}

fn pe(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = PeTask::all();
    let idx = app.sim.task_idx % tasks.len();
    let t = &tasks[idx];
    ui.strong(RichText::new(&t.title).size(16.0));
    code_block(ui, t.bytes.lines());
    ui.add_space(4.0);
    ui.strong(&t.question);
    ui.add_space(4.0);
    choice_task(
        app,
        ui,
        ChoiceTask {
            key: format!("pe{idx}"),
            reward: xp::SIM_PE,
            answers: &t.answers,
            correct: t.correct,
            explain: &t.explain,
            total: tasks.len(),
        },
    );
}

fn oep(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = OepTask::all();
    let idx = app.sim.task_idx % tasks.len();
    let t = &tasks[idx];
    ui.strong(RichText::new(format!("Трассировка упаковщика — задача {}", idx + 1)).size(16.0));
    code_block(ui, t.trace.iter().map(String::as_str));
    ui.add_space(4.0);
    ui.strong(&t.question);
    ui.add_space(4.0);
    choice_task(
        app,
        ui,
        ChoiceTask {
            key: format!("oep{idx}"),
            reward: xp::SIM_OEP,
            answers: &t.answers,
            correct: t.correct,
            explain: &t.explain,
            total: tasks.len(),
        },
    );
}

fn generative(app: &mut AppState, ui: &mut egui::Ui) {
    let (_, today) = now_and_day();
    ui.label(RichText::new("Бесконечные задачи с рандомизацией: каждый «Новый вопрос» — новая задача. Запомнить ответ невозможно — только понять.").weak());
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for (i, kind) in GenKind::ALL.iter().enumerate() {
            if ui
                .selectable_label(app.sim.gen_kind == i, kind.title())
                .clicked()
            {
                app.sim.gen_kind = i;
                app.sim.gen_seed = 0;
            }
        }
    });
    ui.add_space(6.0);
    if app.sim.gen_seed == 0 {
        app.sim.gen_seed = time_seed() | 1;
        app.sim.gen_feedback = None;
        app.sim.gen_answer.clear();
    }
    let task = generate(
        &GenKind::ALL[app.sim.gen_kind % GenKind::ALL.len()],
        app.sim.gen_seed,
    );
    code_block(ui, task.question.lines());
    ui.add_space(6.0);

    let solved = app.sim.gen_feedback.as_ref().is_some_and(|(ok, _)| *ok);
    let (mut check, mut fresh) = (false, false);
    ui.horizontal(|ui| {
        ui.label("Ответ:");
        let edit = egui::TextEdit::singleline(&mut app.sim.gen_answer)
            .desired_width(160.0)
            .hint_text("hex, напр. 1a2b");
        let response = ui.add_enabled(!solved, edit);
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        check = (ui
            .add_enabled(!solved, egui::Button::new("Проверить"))
            .clicked()
            || enter)
            && !solved;
        fresh = ui.button("🎲 Новый вопрос").clicked();
    });
    if check {
        let ok = check_gen(&task, &app.sim.gen_answer);
        app.sim.gen_feedback = Some((ok, task.explain.clone()));
        if ok && app.award_generated(today) {
            let ctx = ui.ctx().clone();
            app.toast(format!("Верно! +{} XP", xp::GENERATED), &ctx);
        }
    }
    if fresh {
        app.sim.gen_seed = 0;
    }
    if let Some((ok, explain)) = &app.sim.gen_feedback {
        ui.add_space(6.0);
        let label = if *ok {
            "✔ Верно!"
        } else {
            "✖ Неверно — попробуйте ещё раз или возьмите новый вопрос"
        };
        ui.label(
            RichText::new(label)
                .color(if *ok { good() } else { warn() })
                .strong(),
        );
        if *ok {
            ui.label(RichText::new(explain).size(12.0));
        }
    }
    let left = app.generated_left(today);
    ui.add_space(8.0);
    let note = if left == 0 {
        "Лимит XP за генератор на сегодня исчерпан — практика продолжается без XP.".to_string()
    } else {
        format!(
            "XP за генератор сегодня: ещё {left} верных ответов (+{} XP каждый).",
            xp::GENERATED
        )
    };
    ui.label(RichText::new(note).weak().size(11.0));
}
