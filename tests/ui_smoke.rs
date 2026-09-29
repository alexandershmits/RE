//! Интерфейс рисуется без паник во всех вкладках, обеих темах и на минимальном размере окна;
//! ни один символ курса и интерфейса не превращается в «квадрат».

use egui::{Context, Pos2, RawInput, Rect};
use re50::state::{AppState, CardSession, PlacementState, Progress, Tab};
use std::collections::BTreeSet;

fn context(light: bool) -> Context {
    let ctx = Context::default();
    let progress = Progress {
        theme: if light { "light" } else { "dark" }.into(),
        ..Progress::default()
    };
    re50::ui::install(&ctx, &progress);
    ctx
}

fn frames(ctx: &Context, app: &mut AppState, size: (f32, f32), count: usize) {
    for _ in 0..count {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| re50::ui::run(app, ui));
        output.textures_delta.clear(); // без окна текстуры никуда не загружаются
    }
}

/// Приложение с историей: часть недель закрыта, есть верные и неверные ответы, ачивки, XP.
fn lived_in() -> AppState {
    let mut app = AppState::in_memory();
    let quizzes = app.curriculum.quizzes.clone();
    for (i, q) in quizzes.iter().take(14).enumerate() {
        app.submit_quiz_answer(
            q,
            if i % 3 == 0 {
                (q.correct + 1) % 4
            } else {
                q.correct
            },
        );
    }
    for w in app.curriculum.weeks.clone().iter().take(3) {
        app.toggle_week_done(&w.id);
        app.complete_pset_with_explain(&format!("{}:0", w.id), "разобрался".into());
    }
    app.progress.xp = 900;
    app.progress.journal = "Заметка о разборе".into();
    app.progress.xp_history = (0..12).map(|d| (20_000 + d, d as u32 * 70)).collect();
    app.progress.time_by_day.insert("20000".into(), 4_000);
    app.progress.bet_results.insert("lv1a".into(), true);
    app.progress.bet_results.insert("lv1b".into(), false);
    app.days_away = 6;
    app
}

#[test]
fn every_tab_renders_in_both_themes_and_sizes() {
    for light in [false, true] {
        let ctx = context(light);
        for size in [(900.0, 600.0), (1180.0, 780.0), (1920.0, 1080.0)] {
            for tab in Tab::ALL {
                let mut app = lived_in();
                app.tab = tab;
                frames(&ctx, &mut app, size, 3);
            }
        }
    }
    re50::ui::apply_theme(&Context::default(), &Progress::default());
}

#[test]
fn every_week_page_renders() {
    let ctx = context(false);
    let mut app = lived_in();
    app.tab = Tab::Course;
    for i in 0..app.curriculum.weeks.len() {
        app.selected_week = i;
        frames(&ctx, &mut app, (1180.0, 780.0), 2);
    }
}

#[test]
fn interactive_states_render() {
    let ctx = context(false);
    let size = (1180.0, 780.0);
    let mut scenarios: Vec<(&str, AppState)> = Vec::new();

    let mut a = lived_in();
    a.tab = Tab::Trainer;
    assert!(a.start_quiz(Some(3)));
    a.quiz.as_mut().unwrap().record(0, false);
    scenarios.push(("квиз: ответ дан", a));

    let mut a = lived_in();
    a.tab = Tab::Cards;
    assert!(a.start_cards(false, 1));
    a.cards.as_mut().unwrap().show_back = true;
    scenarios.push(("карточки: обратная сторона", a));

    let mut a = lived_in();
    a.tab = Tab::Cards;
    a.cards = Some(CardSession::new(Vec::new()));
    scenarios.push(("карточки: сессия завершена", a));

    let mut a = lived_in();
    a.tab = Tab::Reexam;
    a.progress.xp = 900;
    a.start_reexam(3);
    a.reexam_answer(0);
    scenarios.push(("ре-экзамен: ответ дан", a));

    let mut a = lived_in();
    a.tab = Tab::Work;
    a.start_work_session(1_000, 5);
    a.work.hypotheses.push("strcmp с константой".into());
    a.work.report[0] = "ELF64, gcc, без упаковки".into();
    scenarios.push(("рабочая сессия", a));

    let mut a = lived_in();
    a.tab = Tab::Work;
    a.progress.work_history.push(re50::state::WorkRecord {
        challenge: "lv1a".into(),
        seconds: 240,
        method_ok: true,
        completeness: 70,
    });
    scenarios.push(("рабочая сессия: лобби с историей", a));

    let mut a = lived_in();
    a.tab = Tab::Opponent;
    a.opponent.verdict = Some((64, "Часть механики раскрыта".into()));
    a.opponent.llm_reply = Some("А что будет, если указатель невалиден?".into());
    a.opponent.ollama_online = Some(true);
    a.progress.opponent_best.insert("PE".into(), 80);
    scenarios.push(("оппонент", a));

    let mut a = lived_in();
    a.tab = Tab::Placement;
    a.placement = Some(PlacementState {
        pos: 2,
        score: 1,
        selected: Some(0),
        answered: true,
    });
    scenarios.push(("placement: вопрос", a));

    let mut a = lived_in();
    a.tab = Tab::Placement;
    let total = a.curriculum.placement.len();
    a.placement = Some(PlacementState {
        pos: total,
        score: total / 2,
        selected: None,
        answered: false,
    });
    scenarios.push(("placement: итог", a));

    for which in 0..4 {
        let mut a = lived_in();
        a.tab = Tab::Sims;
        a.sim.which = which;
        a.sim.checked = true;
        a.sim.choice = Some(1);
        a.sim.show_answer = true;
        a.sim.gen_feedback = Some((true, "пояснение".into()));
        scenarios.push(("симулятор", a));
    }
    for which in 0..4 {
        let mut a = lived_in();
        a.tab = Tab::Drills;
        a.drill.which = which;
        a.drill.checked = true;
        a.drill.choice = Some(0);
        a.drill.show_answer = true;
        scenarios.push(("дрилл", a));
    }

    let mut a = lived_in();
    a.tab = Tab::Challenges;
    a.progress.challenges_solved.insert("lv1a".into());
    a.progress
        .challenge_bets
        .insert("lv1b".into(), "xor".into());
    scenarios.push(("челленджи", a));

    let mut a = lived_in();
    a.search_query = "ghidra".into();
    a.search_results = a.search_course("ghidra");
    a.pset_pending_explain = Some("w0:0".into());
    a.import_dialog = Some("{}".into());
    a.save_error = Some("нет места на диске".into());
    a.toast = Some(("Готово".into(), 100.0));
    a.new_achievements.push("🏅 Тест".into());
    scenarios.push(("окна поверх интерфейса", a));

    for light in [false, true] {
        let ctx = if light { context(true) } else { ctx.clone() };
        for (name, app) in &mut scenarios {
            let mut copy_tab = app.tab;
            frames(&ctx, app, size, 3);
            frames(&ctx, app, (900.0, 600.0), 2);
            std::mem::swap(&mut copy_tab, &mut app.tab);
            assert!(
                app.tab == copy_tab || *name == "окна поверх интерфейса",
                "{name}: вкладка изменилась сама по себе"
            );
        }
    }
    re50::ui::apply_theme(&Context::default(), &Progress::default());
}

/// Все символы из данных курса и текстов интерфейса должны иметь глиф в шрифтах приложения.
/// Проверка идёт по таблицам символов (cmap) самих шрифтов: `Fonts::has_glyph` в egui даёт ложные
/// отрицательные ответы для символов, которые живут в том же шрифте, что и «символ-заменитель».
#[test]
fn every_character_has_a_glyph() {
    fn strings(v: &serde_json::Value, code: bool, out: &mut Vec<(String, bool)>) {
        match v {
            serde_json::Value::String(s) => out.push((s.clone(), code)),
            serde_json::Value::Array(a) => a.iter().for_each(|x| strings(x, code, out)),
            serde_json::Value::Object(o) => {
                for (k, x) in o {
                    strings(
                        x,
                        code || matches!(
                            k.as_str(),
                            "code" | "topic_map" | "asm" | "bytes" | "trace"
                        ),
                        out,
                    );
                }
            }
            _ => {}
        }
    }
    fn rust_sources(dir: &std::path::Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_sources(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs")
                && !path.ends_with("challenge_blob.rs")
            {
                out.push_str(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    fn coverage(
        defs: &egui::FontDefinitions,
        family: egui::FontFamily,
    ) -> Vec<ttf_parser::Face<'_>> {
        defs.families[&family]
            .iter()
            .map(|name| {
                let data = &defs.font_data[name];
                ttf_parser::Face::parse(&data.font, data.index).expect("шрифт разбирается")
            })
            .collect()
    }

    let mut texts: Vec<(String, bool)> = Vec::new();
    for raw in [
        include_str!("../assets/curriculum.json"),
        include_str!("../assets/drills.json"),
    ] {
        strings(&serde_json::from_str(raw).unwrap(), false, &mut texts);
    }
    let mut source = String::new();
    rust_sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut source,
    );
    texts.push((source, false));

    let defs = re50::ui::theme::font_definitions();
    let proportional = coverage(&defs, egui::FontFamily::Proportional);
    let monospace = coverage(&defs, egui::FontFamily::Monospace);
    let has =
        |faces: &[ttf_parser::Face], c: char| faces.iter().any(|f| f.glyph_index(c).is_some());
    let (mut missing_text, mut missing_code) = (BTreeSet::new(), BTreeSet::new());
    for (text, code) in &texts {
        for c in text
            .chars()
            .filter(|c| !c.is_control() && !c.is_whitespace())
        {
            if *code && !has(&monospace, c) {
                missing_code.insert(c);
            } else if !*code && !has(&proportional, c) && !has(&monospace, c) {
                missing_text.insert(c);
            }
        }
    }
    let show = |set: &BTreeSet<char>| {
        set.iter()
            .map(|c| format!("{c} U+{:04X}", *c as u32))
            .collect::<Vec<_>>()
            .join(", ")
    };
    assert!(
        missing_text.is_empty(),
        "в тексте есть символы без глифа (рисуются квадратом): {}",
        show(&missing_text)
    );
    assert!(
        missing_code.is_empty(),
        "в блоках кода есть символы без глифа моноширинного шрифта: {}",
        show(&missing_code)
    );
    let plain = |c: char| has(&proportional, c);
    assert!(
        plain('→') && plain('▶') && plain('🩸') && plain('я'),
        "основные символы интерфейса должны рисоваться прежде всего пропорциональным шрифтом"
    );
}
