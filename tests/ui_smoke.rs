//! Интерфейс рисуется без паник во всех вкладках, обеих темах и на минимальном размере окна;
//! ни один символ курса и интерфейса не превращается в «квадрат».

use egui::{Color32, Context, Pos2, RawInput, Rect};
use re50::state::{AppState, CardSession, PlacementState, Progress, Tab};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU32, Ordering};

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
        let output = frame(ctx, app, window(size));
        let warnings = warning_texts(&output.shapes);
        assert!(
            warnings.is_empty(),
            "egui показал предупреждение вместо интерфейса: {warnings:?}"
        );
        let raw = raw_markup_texts(&output.shapes);
        assert!(raw.is_empty(), "на экране остались знаки разметки: {raw:?}");
        let panel = ctx.global_style().visuals.panel_fill;
        let faint = unreadable_texts(&output.shapes, panel);
        assert!(
            faint.is_empty(),
            "текст почти не виден (контраст < {MIN_CONTRAST}): {faint:?}"
        );
    }
}

/// egui в debug-сборке рисует красные «First use of widget ID …» при совпадении идентификаторов:
/// такие виджеты делят состояние (раскрытие, фокус), поэтому это ошибка интерфейса.
fn warning_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    shown_texts(shapes)
        .into_iter()
        .filter(|t| {
            ["First use of", "Second use of", "Double use of"]
                .iter()
                .any(|w| t.contains(w))
        })
        .collect()
}

/// Тексты, в которых остались знаки разметки курса (`код`, **жирный**): их должен был разобрать рендер.
fn raw_markup_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    shown_texts(shapes)
        .into_iter()
        .filter(|t| t.matches('`').count() >= 2 || t.matches("**").count() >= 2)
        .collect()
}

/// Цвета, которыми egui на самом деле рисует глифы. Отключённый виджет гасится перекраской вершин уже
/// построенной раскладки, а `format.color` в секциях остаётся прежним, поэтому читаем вершины.
fn glyph_colors(t: &egui::epaint::TextShape) -> Vec<Color32> {
    let mut colors = Vec::new();
    for placed in &t.galley.rows {
        let visuals = &placed.row.visuals;
        let Some(glyphs) = visuals
            .mesh
            .vertices
            .get(visuals.glyph_vertex_range.clone())
        else {
            continue;
        };
        for vertex in glyphs {
            let color = t
                .override_text_color
                .unwrap_or(if vertex.color == Color32::PLACEHOLDER {
                    t.fallback_color
                } else {
                    vertex.color
                });
            if !colors.contains(&color) {
                colors.push(color);
            }
        }
    }
    colors
}

/// Текст нарисован частично прозрачным — так egui показывает отключённые виджеты.
fn is_dimmed(t: &egui::epaint::TextShape) -> bool {
    t.opacity_factor < 0.99 || glyph_colors(t).iter().any(|c| c.a() < 255)
}

/// Минимальный контраст текста с подложкой (WCAG для крупного текста и элементов интерфейса).
const MIN_CONTRAST: f64 = 3.0;

/// Цвет `src` поверх непрозрачного `dst` (egui хранит цвета с предумноженной альфой).
fn over(src: Color32, dst: Color32) -> Color32 {
    let keep = 255 - u16::from(src.a());
    let mix = |s: u8, d: u8| (u16::from(s) + u16::from(d) * keep / 255).min(255) as u8;
    Color32::from_rgb(
        mix(src.r(), dst.r()),
        mix(src.g(), dst.g()),
        mix(src.b(), dst.b()),
    )
}

/// Тексты, которые egui нарисовал с контрастом ниже `MIN_CONTRAST` относительно того, что под ними.
/// Фигуры идут в порядке отрисовки, поэтому подложка — последний закрашенный прямоугольник под точкой.
/// Так находится и белый текст на светлой полосе прогресса, и текст на тонированной карточке.
fn unreadable_texts(shapes: &[egui::epaint::ClippedShape], panel: Color32) -> Vec<String> {
    struct Fill {
        clip: Rect,
        rect: Rect,
        color: Color32,
    }
    fn under(fills: &[Fill], at: Pos2, panel: Color32) -> Color32 {
        let Some(i) = fills
            .iter()
            .rposition(|f| f.clip.contains(at) && f.rect.contains(at))
        else {
            return panel;
        };
        if fills[i].color.a() == 255 {
            fills[i].color
        } else {
            over(fills[i].color, under(&fills[..i], at, panel))
        }
    }
    fn walk(
        shape: &egui::epaint::Shape,
        clip: Rect,
        fills: &mut Vec<Fill>,
        panel: Color32,
        out: &mut Vec<String>,
    ) {
        use egui::epaint::Shape;
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, clip, fills, panel, out)),
            Shape::Rect(r) if r.fill.a() > 0 => fills.push(Fill {
                clip,
                rect: r.rect,
                color: r.fill,
            }),
            // Отключённые виджеты egui рисует полупрозрачными намеренно; то, что должно читаться (ответы
            // квиза), проверяет `answered_quiz_options_are_not_dimmed`.
            Shape::Text(t) if !is_dimmed(t) => {
                let rect = t.galley.rect.translate(t.pos.to_vec2());
                if !clip.intersects(rect) {
                    return; // за пределами видимой области (прокрутка)
                }
                let points = [
                    rect.left_center() + egui::vec2(2.0, 0.0),
                    rect.center(),
                    rect.right_center() - egui::vec2(2.0, 0.0),
                ];
                for glyphs in glyph_colors(t) {
                    for at in points {
                        let bg = under(fills, at, panel);
                        let ratio = re50::ui::theme::contrast_ratio(glyphs, bg);
                        if ratio < MIN_CONTRAST {
                            out.push(format!(
                                "«{}»: {glyphs:?} на {bg:?} = {ratio:.2}",
                                t.galley.text().trim()
                            ));
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let (mut fills, mut out) = (Vec::new(), Vec::new());
    for clipped in shapes {
        walk(
            &clipped.shape,
            clipped.clip_rect,
            &mut fills,
            panel,
            &mut out,
        );
    }
    out
}

fn shown_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn walk(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_string()),
            egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|s| walk(&s.shape, &mut out));
    out
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

fn frame(ctx: &Context, app: &mut AppState, input: RawInput) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, |ui| re50::ui::run(app, ui));
    output.textures_delta.clear(); // без окна текстуры никуда не загружаются
    output
}

/// Ввод одного кадра. Время идёт по 0.25 с на кадр: без часов окно «Новая ачивка» и другие плавные
/// появления навсегда остались бы на первом, почти прозрачном кадре.
fn window(size: (f32, f32)) -> RawInput {
    static FRAMES: AtomicU32 = AtomicU32::new(0);
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(size.0, size.1))),
        time: Some(f64::from(FRAMES.fetch_add(1, Ordering::Relaxed)) * 0.25),
        ..RawInput::default()
    }
}

#[test]
fn startup_notice_stays_on_screen_until_it_is_dismissed() {
    let ctx = context(false);
    let mut app = AppState::in_memory();
    app.startup_notice = Some("Файл прогресса повреждён".into());
    let mut seen = false;
    for _ in 0..3 {
        let output = frame(&ctx, &mut app, window((1200.0, 800.0)));
        seen |= shown_texts(&output.shapes)
            .iter()
            .any(|t| t.contains("⚠") && t.contains("Файл прогресса повреждён"));
    }
    assert!(seen, "предупреждение должно быть видно");
    assert!(
        app.startup_notice.is_some(),
        "само оно не пропадает: закрывает его пользователь"
    );
}

#[test]
fn saved_theme_wins_over_the_system_theme() {
    // регресс: при светлой ОС и сохранённой тёмной теме egui брал нетронутый «светлый» слот стиля —
    // палитра выходила тёмной на светлых визуалах, а масштаб шрифта пропадал
    for (saved_light, system) in [(false, egui::Theme::Light), (true, egui::Theme::Dark)] {
        let ctx = Context::default();
        let progress = Progress {
            theme: if saved_light { "light" } else { "dark" }.into(),
            font_scale: 1.5,
            ..Progress::default()
        };
        re50::ui::install(&ctx, &progress);
        let mut app = AppState::in_memory();
        app.progress = progress.clone();
        let input = RawInput {
            system_theme: Some(system),
            ..window((1200.0, 800.0))
        };
        frame(&ctx, &mut app, input);
        let style = ctx.global_style();
        assert_eq!(
            style.visuals.dark_mode, !saved_light,
            "сохранена светлая: {saved_light}, тема ОС: {system:?}"
        );
        let body = style.text_styles[&egui::TextStyle::Body].size;
        let plain = egui::Style::default().text_styles[&egui::TextStyle::Body].size;
        assert!(
            (body - plain * 1.5).abs() < 0.01,
            "масштаб шрифта потерян: {body}"
        );
    }
}

#[test]
fn raw_markup_detector_sees_unrendered_marks() {
    let ctx = context(false);
    let mut output = ctx.run_ui(RawInput::default(), |ui| {
        ui.label("вот `код` и **жирный**");
        ui.label("обычная строка и char **argv");
    });
    output.textures_delta.clear();
    let raw = raw_markup_texts(&output.shapes);
    assert_eq!(raw, vec!["вот `код` и **жирный**".to_string()]);
}

/// Исходники `.rs` из каталога (без вшитых бинарей) одной строкой.
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

#[test]
fn contrast_check_sees_white_text_on_an_empty_progress_bar() {
    // регресс: подпись внутри полосы egui красит белым, а пустая полоса в светлой теме светло-серая
    let ctx = context(true);
    let mut output = ctx.run_ui(window((600.0, 200.0)), |ui| {
        ui.add(egui::ProgressBar::new(0.0).text("0/12"));
        ui.label("обычная подпись рядом");
    });
    output.textures_delta.clear();
    let faint = unreadable_texts(&output.shapes, ctx.global_style().visuals.panel_fill);
    assert_eq!(faint.len(), 1, "{faint:?}");
    assert!(faint[0].contains("0/12"), "{faint:?}");
    re50::ui::apply_theme(&Context::default(), &Progress::default());
}

/// Тексты, которые egui нарисовал частично прозрачными (так выглядят отключённые виджеты).
fn dimmed_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn walk(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
        use egui::epaint::Shape;
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Text(t) if is_dimmed(t) => out.push(t.galley.text().to_string()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|s| walk(&s.shape, &mut out));
    out
}

#[test]
fn answered_quiz_options_are_not_dimmed() {
    // регресс: после ответа кнопки вариантов отключались, и egui рисовал все варианты, включая
    // зелёный правильный и красный выбранный, вполовину прозрачными
    for light in [false, true] {
        let ctx = context(light);
        let mut app = AppState::in_memory();
        assert!(app.start_quiz(Some(3)));
        let before = frame(&ctx, &mut app, window((1180.0, 780.0)));
        assert!(dimmed_texts(&before.shapes).is_empty());
        app.tab = Tab::Trainer;
        app.quiz.as_mut().unwrap().record(0, false);
        let after = frame(&ctx, &mut app, window((1180.0, 780.0)));
        let options: Vec<String> = shown_texts(&after.shapes)
            .into_iter()
            .filter(|t| {
                t.starts_with("A) ")
                    || t.starts_with("B) ")
                    || t.starts_with("C) ")
                    || t.starts_with("D) ")
            })
            .collect();
        assert_eq!(options.len(), 4, "{options:?}");
        let dimmed = dimmed_texts(&after.shapes);
        assert!(
            options.iter().all(|o| !dimmed.contains(o)),
            "после ответа варианты нарисованы полупрозрачными (светлая тема: {light}): {dimmed:?}"
        );
    }
    re50::ui::apply_theme(&Context::default(), &Progress::default());
}

#[test]
fn progress_bars_keep_their_label_outside() {
    let mut text = String::new();
    rust_sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"),
        &mut text,
    );
    let widgets = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/widgets.rs"),
    )
    .unwrap();
    // сама функция-помощник живёт в widgets.rs и подписи в полосу не кладёт
    let text = text.replace(&widgets, "");
    for statement in text.split("ProgressBar::new(").skip(1) {
        let statement = statement.split(';').next().unwrap_or_default();
        assert!(
            !statement.contains(".text(") && !statement.contains(".show_percentage("),
            "подпись внутри полосы нечитаема в светлой теме — используйте widgets::labeled_progress: …{}",
            statement.chars().take(80).collect::<String>()
        );
    }
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
    fn coverage(
        defs: &egui::FontDefinitions,
        family: egui::FontFamily,
    ) -> Vec<skrifa::FontRef<'_>> {
        defs.families[&family]
            .iter()
            .map(|name| {
                let data = &defs.font_data[name];
                skrifa::FontRef::from_index(&data.font, data.index).expect("шрифт разбирается")
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
    let has = |faces: &[skrifa::FontRef], c: char| {
        use skrifa::MetadataProvider;
        faces
            .iter()
            .any(|f| f.charmap().map(c).is_some_and(|glyph| glyph.to_u32() != 0))
    };
    // контроль самой проверки: настоящий символ есть, несуществующий — нет
    assert!(has(&proportional, 'Ж') && has(&monospace, 'x'));
    assert!(!has(&proportional, '\u{10FFFE}') && !has(&monospace, '\u{10FFFE}'));
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

/// Положительный контроль: детектор действительно видит совпадение идентификаторов
/// (без него тест на «нет предупреждений» мог бы проходить вхолостую).
#[test]
fn id_clash_detector_sees_duplicate_widgets() {
    let ctx = context(false);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(600.0, 400.0))),
        ..RawInput::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        ui.collapsing("одинаково", |ui| ui.label("a"));
        ui.collapsing("одинаково", |ui| ui.label("b"));
    });
    output.textures_delta.clear();
    assert!(
        !warning_texts(&output.shapes).is_empty(),
        "детектор не заметил совпадающие id"
    );
}

#[test]
fn parallel_tests_do_not_share_a_theme() {
    // регресс: флаг темы был общим для процесса, и тесты в разных потоках видели палитру друг друга
    // (светлый акцент на тёмной подложке); теперь тема принадлежит потоку
    let handles: Vec<_> = [false, true, false, true]
        .into_iter()
        .map(|light| {
            std::thread::spawn(move || {
                let _ctx = context(light);
                for _ in 0..200 {
                    assert_eq!(!re50::ui::theme::is_dark(), light);
                    std::thread::yield_now();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("поток видел чужую тему");
    }
}
