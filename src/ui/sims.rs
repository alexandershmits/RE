use eframe::egui::{self, RichText, ScrollArea};

use super::{ACCENT, GOOD, WARN};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.sim.which == 3 {
            sims_generative(app, ui);
            return;
        }
        ui.heading("⚙️ Микро-симуляторы");
        ui.label(RichText::new("Потрогайте теорию руками, не открывая отладчик: предскажи регистры, распарси PE-байты, найди OEP.").weak());
        ui.separator();
        ui.horizontal(|ui| {
            for (i, name) in ["🧮 Регистры и инструкции", "📦 PE-байты", "🔍 Поиск OEP", "🎲 Бесконечный генератор"].iter().enumerate() {
                if ui.selectable_label(app.sim.which == i, *name).clicked() {
                    app.sim = Default::default();
                    app.sim.which = i;
                }
            }
        });
        ui.add_space(8.0);
        ScrollArea::vertical().id_salt("sims").show(ui, |ui| {
            match app.sim.which {
                0 => sim_regs(app, ui),
                1 => sim_pe(app, ui),
                _ => sim_oep(app, ui),
            }
        });
    });
}

fn sim_regs(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = crate::simulators::RegTask::all();
    if app.sim.task_idx >= tasks.len() {
        app.sim.task_idx = 0;
    }
    let task = &tasks[app.sim.task_idx];

    ui.strong(RichText::new(&task.title).size(16.0));
    ui.label(
        RichText::new("Введите hex-значения (без 0x) для каждого регистра ПОСЛЕ выполнения кода:")
            .weak(),
    );
    ui.add_space(4.0);
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(12, 13, 18))
        .show(ui, |ui| {
            for line in &task.code {
                ui.monospace(format!("    {line}"));
            }
        });
    ui.add_space(6.0);

    let mut submit: Option<Vec<(String, u64)>> = None;
    egui::Grid::new("reginputs").num_columns(3).show(ui, |ui| {
        let mut keys: Vec<String> = task.init.iter().map(|(r, _)| r.to_string()).collect();
        keys.sort();
        for k in keys {
            let mut v = app.sim.user_input.entry(k.clone()).or_default().clone();
            ui.label(format!("{k} = 0x"));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut v)
                    .desired_width(120.0)
                    .font(egui::TextStyle::Monospace),
            );
            app.sim.user_input.insert(k.clone(), v);
            if resp.changed() {
                app.sim.checked = false;
            }
            ui.end_row();
        }
    });
    ui.horizontal(|ui| {
        if ui.button("✔ Проверить").clicked() {
            let mut vals = Vec::new();
            for (r, _) in &task.answers {
                let s = app.sim.user_input.get(*r).cloned().unwrap_or_default();
                let s = s.trim().trim_start_matches("0x").to_string();
                vals.push((
                    r.to_string(),
                    u64::from_str_radix(&s, 16).unwrap_or(u64::MAX),
                ));
            }
            submit = Some(vals);
        }
        if ui.button("➡ Следующая задача").clicked() {
            app.sim.task_idx = (app.sim.task_idx + 1) % tasks.len();
            app.sim.user_input.clear();
            app.sim.checked = false;
            app.sim.show_answer = false;
        }
        if ui.button("💡 Ответ").clicked() {
            app.sim.show_answer = !app.sim.show_answer;
        }
    });

    if let Some(vals) = submit {
        let all_ok = vals
            .iter()
            .all(|(r, v)| task.answers.iter().any(|(ar, av)| *ar == r && av == v));
        app.sim.checked = true;
        app.sim.last_ok = all_ok;
        if all_ok {
            app.sim.solved.insert(format!("reg{}", app.sim.task_idx));
            app.add_xp(15);
            app.toast("Верно! +15 XP", ui.ctx());
        }
        app.save();
    }

    if app.sim.checked {
        if app.sim.last_ok {
            ui.label(RichText::new("✔ Всё верно!").color(GOOD).strong());
        } else {
            ui.label(
                RichText::new("✘ Есть ошибки — проверьте ещё раз или посмотрите ответ.")
                    .color(ACCENT)
                    .strong(),
            );
        }
    }
    if app.sim.show_answer {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            for (r, v) in &task.answers {
                ui.monospace(format!("{r} = 0x{v:X}"));
            }
            ui.label(RichText::new(&task.explain).weak());
        });
    }
}

fn sim_pe(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = crate::simulators::PeTask::all();
    if app.sim.task_idx >= tasks.len() {
        app.sim.task_idx = 0;
    }
    let idx = app.sim.task_idx;
    let task = &tasks[idx];

    ui.strong(RichText::new(&task.title).size(16.0));
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(12, 13, 18))
        .show(ui, |ui| {
            ui.monospace(&task.bytes);
        });
    ui.add_space(4.0);
    ui.strong(&task.question);
    ui.add_space(4.0);

    let mut click: Option<usize> = None;
    for (i, a) in task.answers.iter().enumerate() {
        let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
        if app.sim.checked {
            if i == task.correct {
                text = text.color(GOOD).strong();
            } else if app.sim.choice == Some(i) {
                text = text.color(ACCENT).strong();
            }
        }
        if ui
            .add_enabled(
                !app.sim.checked,
                egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap),
            )
            .clicked()
        {
            click = Some(i);
        }
    }
    if let Some(c) = click {
        app.sim.choice = Some(c);
        app.sim.checked = true;
        app.sim.last_ok = c == task.correct;
        if app.sim.last_ok {
            app.sim.solved.insert(format!("pe{idx}"));
            app.add_xp(15);
            app.toast("Верно! +15 XP", ui.ctx());
        }
        app.save();
    }
    if app.sim.checked {
        if app.sim.last_ok {
            ui.label(RichText::new("✔ Верно!").color(GOOD).strong());
        } else {
            ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong());
        }
        ui.label(RichText::new(format!("💡 {}", task.explain)).weak());
    }
    ui.horizontal(|ui| {
        if ui.button("➡ Следующая").clicked() {
            app.sim.task_idx = (app.sim.task_idx + 1) % tasks.len();
            app.sim.checked = false;
            app.sim.choice = None;
        }
    });
}

fn sim_oep(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = crate::simulators::OepTask::all();
    if app.sim.task_idx >= tasks.len() {
        app.sim.task_idx = 0;
    }
    let idx = app.sim.task_idx;
    let task = &tasks[idx];

    ui.strong(RichText::new(format!("Трассировка упаковщика — задача {}", idx + 1)).size(16.0));
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(12, 13, 18))
        .show(ui, |ui| {
            for line in &task.trace {
                ui.monospace(line);
            }
        });
    ui.add_space(4.0);
    ui.strong(&task.question);
    ui.add_space(4.0);

    let mut click: Option<usize> = None;
    for (i, a) in task.answers.iter().enumerate() {
        let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
        if app.sim.checked {
            if i == task.correct {
                text = text.color(GOOD).strong();
            } else if app.sim.choice == Some(i) {
                text = text.color(ACCENT).strong();
            }
        }
        if ui
            .add_enabled(
                !app.sim.checked,
                egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap),
            )
            .clicked()
        {
            click = Some(i);
        }
    }
    if let Some(c) = click {
        app.sim.choice = Some(c);
        app.sim.checked = true;
        app.sim.last_ok = c == task.correct;
        if app.sim.last_ok {
            app.sim.solved.insert(format!("oep{idx}"));
            app.add_xp(20);
            app.toast("Верно! +20 XP", ui.ctx());
        }
        app.save();
    }
    if app.sim.checked {
        if app.sim.last_ok {
            ui.label(RichText::new("✔ Верно!").color(GOOD).strong());
        } else {
            ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong());
        }
        ui.label(RichText::new(format!("💡 {}", task.explain)).weak());
    }
    ui.horizontal(|ui| {
        if ui.button("➡ Следующая").clicked() {
            app.sim.task_idx = (app.sim.task_idx + 1) % tasks.len();
            app.sim.checked = false;
            app.sim.choice = None;
        }
    });
}

fn sims_generative(app: &mut AppState, ui: &mut egui::Ui) {
    use crate::simulators::{check_gen, generate, GenKind};
    ui.add_space(8.0);
    ui.label(RichText::new("Бесконечные задачи с рандомизацией: каждый «Новый вопрос» — новая задача. Запомнить ответы невозможно — работает только навык.").weak());

    let kinds = [
        GenKind::RipRelative,
        GenKind::LittleEndian,
        GenKind::DecodeMov,
    ];
    ui.horizontal(|ui| {
        for (i, k) in kinds.iter().enumerate() {
            if ui
                .selectable_label(app.sim.gen_kind == i, k.title())
                .clicked()
            {
                app.sim.gen_kind = i;
                app.sim.gen_seed = 0;
                app.sim.gen_feedback = None;
                app.sim.gen_answer.clear();
            }
        }
    });
    ui.add_space(6.0);

    // Текущая задача генерируется детерминированно из seed (0 = сгенерировать)
    if app.sim.gen_seed == 0 {
        app.sim.gen_seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1);
        app.sim.gen_feedback = None;
        app.sim.gen_answer.clear();
    }
    let task = generate(&kinds[app.sim.gen_kind], app.sim.gen_seed);

    ui.group(|ui| {
        ui.label(RichText::new(&task.question).size(15.0).monospace());
    });
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Ответ:");
        ui.add(
            egui::TextEdit::singleline(&mut app.sim.gen_answer)
                .desired_width(160.0)
                .hint_text("hex, напр. 1a2b"),
        );
        if ui.add(egui::Button::new("Проверить")).clicked() {
            let ok = check_gen(&task, &app.sim.gen_answer);
            app.sim.gen_feedback = Some((ok, task.explain.clone()));
            if ok {
                app.add_xp(10);
            }
        }
        if ui.button("🎲 Новый вопрос").clicked() {
            app.sim.gen_seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(2);
            app.sim.gen_feedback = None;
            app.sim.gen_answer.clear();
        }
    });

    if let Some((ok, explain)) = &app.sim.gen_feedback {
        ui.add_space(6.0);
        ui.label(
            RichText::new(if *ok {
                "✔ Верно! +10 XP"
            } else {
                "✘ Неверно"
            })
            .color(if *ok { GOOD } else { WARN })
            .strong(),
        );
        ui.label(RichText::new(explain).size(12.0));
    }
}
