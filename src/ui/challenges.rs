use super::theme::{accent, good, warn};
use crate::curriculum::Challenge;
use crate::jobs::Job;
use crate::state::{xp, AppState, GeneratorReport};
use eframe::egui::{self, RichText, ScrollArea};
use std::path::Path;
use std::process::Command;

const LEVEL_NAMES: [&str; 5] = [
    "🥉 Уровень 1 — строки и константы",
    "🥈 Уровень 2 — арифметика и байты",
    "🥇 Уровень 3 — алгоритмы и хеши",
    "🏅 Уровень 4 — трансформации и ключгены",
    "🏆 Уровень 5 — специализация (.NET/Go/IL)",
];

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let cur = app.curriculum.clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading("🚩 Встроенные челленджи");
        generator_row(app, ui, &ctx);
        ui.label(RichText::new("Та же логика проверки, но пароль, ключ и маска рандомизируются при каждой генерации: запомнить ответ из райтапа невозможно — работает только понимание. Нужны Python 3 и gcc (для .exe — mingw).").weak().size(12.0));
        ui.add_space(6.0);
        ui.label(RichText::new(format!(
            "{} учебных crackmes (Linux x86-64 ELF и Windows PE), собранных специально для курса. Экспортируйте бинари в лабу, решите в Ghidra/x64dbg и введите флаг — приложение проверит. +{} XP за флаг, подсказки внутри.",
            cur.challenges.len(), xp::FLAG
        )).weak());
        ui.separator();
        ScrollArea::vertical().id_salt("challenges").show(ui, |ui| {
            let mut last_level = 0;
            for ch in &cur.challenges {
                if ch.level != last_level {
                    ui.add_space(6.0);
                    let name = LEVEL_NAMES.get(usize::from(ch.level).saturating_sub(1)).copied().unwrap_or("Дополнительный уровень");
                    ui.strong(RichText::new(name).color(accent()).size(16.0));
                    last_level = ch.level;
                }
                card(app, ui, &ctx, ch);
                ui.add_space(4.0);
            }
            ui.add_space(10.0);
            ui.strong("📝 Эталонные write-ups (как выглядит хорошо):");
            for e in &cur.ethalon_writeups {
                ui.collapsing(RichText::new(&e.pset).strong(), |ui| {
                    ui.label(&e.lesson);
                });
            }
        });
    });
}

fn generator_row(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    let running = app.generator.is_some();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("🎲 Adversarial loop:").strong());
        let button = egui::Button::new("Сгенерировать новый вариант (случайный флаг)").small();
        if ui.add_enabled(!running, button).clicked() {
            match app
                .paths
                .export_root()
                .map(|root| root.join("re50-generated"))
            {
                Ok(dir) => app.generator = Some(Job::spawn(ctx, move || run_generator(&dir))),
                Err(e) => app.toast(format!("Ошибка: {e}"), ctx),
            }
        }
        if running {
            ui.spinner();
            ui.label(RichText::new("генерация…").weak());
        }
    });
}

/// Запускает вшитый Python-генератор в фоне. Пробует `python3`, `python`, `py -3` (Windows).
fn run_generator(dir: &Path) -> GeneratorReport {
    let fail = |message: String| GeneratorReport { ok: false, message };
    let script = std::env::temp_dir().join("re50_challenge_generator.py");
    if let Err(e) = std::fs::write(&script, crate::generator_script::GENERATOR_PY) {
        return fail(format!(
            "Ошибка: не удалось записать скрипт генератора: {e}"
        ));
    }
    let mut last_error =
        "Python 3 не найден: установите его (python.org) и gcc, затем повторите.".to_string();
    for (program, prefix) in [
        ("python3", &[][..]),
        ("python", &[][..]),
        ("py", &["-3"][..]),
    ] {
        match Command::new(program)
            .args(prefix)
            .arg(&script)
            .arg("all")
            .arg(dir)
            .output()
        {
            Ok(out) if out.status.success() => {
                return GeneratorReport {
                    ok: true,
                    message: format!(
                        "🎲 5 новых челленджей сгенерированы в {} — флаги в .meta.json (не подглядывать, пока не решили!)",
                        dir.display()
                    ),
                };
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let tail: String = stderr
                    .trim()
                    .chars()
                    .rev()
                    .take(300)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                last_error = format!("Ошибка генератора (нужны Python 3 и gcc): {tail}");
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => last_error = format!("Ошибка запуска {program}: {e}"),
        }
    }
    fail(last_error)
}

fn card(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context, ch: &Challenge) {
    let solved = app.progress.challenges_solved.contains(&ch.id);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(if solved { "✅" } else { "▫" }).size(16.0));
            ui.strong(&ch.title);
            ui.label(RichText::new(format!("({})", ch.id)).weak().size(11.0));
            if solved {
                ui.label(
                    RichText::new(format!("FLAG{{{}}}", ch.flag))
                        .color(good())
                        .size(11.0),
                );
            }
        });
        ui.label(RichText::new(&ch.desc).weak().size(12.0));
        ui.collapsing("💭 Подсказка", |ui| {
            ui.label(&ch.hint);
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .small_button("📤 В лабу")
                .on_hover_text("Скопировать бинари в папку re50-lab")
                .clicked()
            {
                match app.export_challenge(&ch.id) {
                    Ok(dir) => app.toast(format!("Скопировано в {dir}"), ctx),
                    Err(e) => app.toast(format!("Не удалось скопировать: {e}"), ctx),
                }
            }
            ui.label(
                RichText::new(format!(
                    "{0} / {0}.exe · SHA-256 {1}… / {2}…",
                    ch.id, ch.sha256, ch.sha256_exe
                ))
                .weak()
                .size(10.0),
            );
        });
        bet(app, ui, ctx, ch, solved);
        if !solved {
            flag_input(app, ui, ctx, ch);
        }
    });
}

/// Режим «Ставка»: гипотеза записывается ДО решения, оценивается после.
fn bet(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context, ch: &Challenge, solved: bool) {
    if let Some(&hit) = app.progress.bet_results.get(&ch.id) {
        let text = if hit {
            "🎯 Ставка: угадал механизм ✔"
        } else {
            "🎯 Ставка: мимо — выводы в журнал"
        };
        ui.label(
            RichText::new(text)
                .color(if hit { good() } else { warn() })
                .size(11.0),
        );
        return;
    }
    let has_bet = app.progress.challenge_bets.contains_key(&ch.id);
    let title = if has_bet {
        "🎯 Ставка сделана — изменить"
    } else {
        "🎯 Ставка: напиши гипотезу ДО решения"
    };
    ui.collapsing(title, |ui| {
        ui.label(RichText::new("Напиши, где и как проверяется пароль — до того, как решишь. После решения отметь, угадал ли механизм: так калибруется интуиция.").weak().size(11.0));
        let mut text = app.progress.challenge_bets.get(&ch.id).cloned().unwrap_or_default();
        let edit = egui::TextEdit::multiline(&mut text)
            .desired_rows(3)
            .hint_text("Например: «ожидаю цикл по байтам с xor 0x42 и сравнение через memcmp в конце»");
        if ui.add(edit).changed() {
            app.progress.challenge_bets.insert(ch.id.clone(), text);
            app.mark_dirty();
        }
        if has_bet && solved {
            for (label, hit, note) in [
                ("✔ Сверить: гипотеза верна?", true, "Ставка зафиксирована. Точность интуиции растёт!"),
                ("✖ Сверить: гипотеза мимо", false, "Мимо — это тоже данные. Запиши в журнал, где ошиблась интуиция."),
            ] {
                if ui.small_button(label).clicked() {
                    app.progress.bet_results.insert(ch.id.clone(), hit);
                    app.mark_dirty();
                    app.toast(note, ctx);
                }
            }
        }
    });
}

fn flag_input(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context, ch: &Challenge) {
    ui.horizontal(|ui| {
        ui.label("Флаг:");
        let input = app.challenge_input.entry(ch.id.clone()).or_default();
        let edit = egui::TextEdit::singleline(input)
            .hint_text("FLAG{...} или содержимое")
            .desired_width(220.0);
        let response = ui.add(edit);
        // раньше проверка запускалась от простого клика по полю — пустой флаг «сдавался» при каждом фокусе
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Проверить").clicked() || enter {
            let flag = app.challenge_input.get(&ch.id).cloned().unwrap_or_default();
            if app.submit_flag(&ch.id, &flag) {
                app.toast(format!("🚩 Верно! +{} XP", xp::FLAG), ctx);
            } else {
                app.toast("Неверно — вернитесь к дизассемблеру", ctx);
            }
        }
    });
}
