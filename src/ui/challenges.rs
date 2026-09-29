use eframe::egui::{self, RichText, ScrollArea};

use super::{ACCENT, GOOD, WARN};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🚩 Встроенные челленджи");
        // Adversarial loop: генерация нового варианта
        ui.horizontal(|ui| {
            ui.label(RichText::new("🎲 Adversarial loop:").strong());
            if ui.small_button("Сгенерировать новый вариант (случайный флаг)").clicked() {
                let lab = format!("{}/re50-generated", std::env::var("HOME").unwrap_or_default());
                // вшитый скрипт -> temp file -> python3
                let tmp = std::env::temp_dir().join("re50_challenge_generator.py");
                let _ = std::fs::write(&tmp, crate::generator_script::GENERATOR_PY);
                let out = std::process::Command::new("python3")
                    .arg(&tmp).arg("all").arg(&lab)
                    .output();
                app.toast = Some((match out {
                    Ok(o) if o.status.success() => format!(
                        "🎲 5 новых челленджей сгенерированы в {} — флаги в .meta.json (не подглядывать, пока не решили!)", lab),
                    Ok(o) => format!("Ошибка генератора: {}", String::from_utf8_lossy(&o.stderr)),
                    Err(e) => format!("python3 не найден: {e}"),
                }, 10.0));
            }
        });
        ui.label(RichText::new("Та же логика проверки — но пароль/ключ/маска рандомизируются при каждой генерации. Запомнить ответ из райтапа невозможно: работает только понимание.").weak().size(12.0));
        ui.add_space(6.0);
        ui.label(RichText::new(
            "15 учебных crackmes (Linux x86-64 ELF), собранных специально для курса.              Бинари лежат в assets/challenges/. Решите в Ghidra/x64dbg, введите флаг — приложение проверит.              +50 XP за флаг, подсказки внутри.")
            .weak());
        ui.separator();
        ScrollArea::vertical().id_salt("challenges").show(ui, |ui| {
            let mut last_level = 0;
            for ch in app.curriculum.challenges.clone() {
                if ch.level != last_level {
                    let names = ["", "🥉 Уровень 1 — строки и константы", "🥈 Уровень 2 — арифметика и байты",
                                 "🥇 Уровень 3 — алгоритмы и хеши", "🏅 Уровень 4 — трансформации и ключгены",
                                 "🏆 Уровень 5 — специализация (.NET/Go/IL)"];
                    ui.add_space(6.0);
                    ui.strong(RichText::new(names[ch.level as usize]).color(ACCENT).size(16.0));
                    last_level = ch.level;
                }
                let solved = app.progress.challenges_solved.contains(&ch.id);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width() - 8.0);
                    ui.horizontal(|ui| {
                        let mark = if solved { RichText::new("✅").size(16.0) } else { RichText::new("▫️").size(16.0) };
                        ui.label(mark);
                        ui.strong(&ch.title);
                        ui.label(RichText::new(format!("({})", ch.id)).weak().size(11.0));
                        if solved {
                            ui.label(RichText::new(format!("FLAG{{{}}}", ch.flag)).color(GOOD).size(11.0));
                        }
                    });
                    ui.label(RichText::new(&ch.desc).weak().size(12.0));
                    ui.collapsing("💭 Подсказка", |ui| {
                        ui.label(&ch.hint);
                    });
                    ui.horizontal(|ui| {
                        if ui.small_button("📤 В лабу").on_hover_text("Скопировать бинари в ~/re50-lab/").clicked() {
                            match app.export_challenge(&ch.id) {
                                Some(dir) => app.toast(format!("Скопировано в {dir}"), ctx),
                                None => app.toast("Не удалось скопировать", ctx),
                            }
                        }
                        ui.label(RichText::new(format!("файлы: {} / {}.exe", ch.id, ch.id)).weak().size(10.0));
                    });
                    // 🎯 Режим «Ставка»: гипотеза ДО решения
                    {
                        let has_bet = app.progress.challenge_bets.contains_key(&ch.id);
                        let bet_done = app.progress.bet_results.contains_key(&ch.id);
                        if !bet_done {
                            ui.collapsing(
                                if has_bet { "🎯 Ставка сделана — изменить" } else { "🎯 Ставка: напиши гипотезу ДО решения" },
                                |ui| {
                                    ui.label(RichText::new(
                                        "Напиши, где и как проверяется пароль — до того, как решишь. После решения сравнишь гипотезу с реальностью. Так калибруется профессиональная интуиция."
                                    ).weak().size(11.0));
                                    let mut bet = app.progress.challenge_bets.get(&ch.id).cloned().unwrap_or_default();
                                    let resp = egui::TextEdit::multiline(&mut bet)
                                        .desired_rows(3)
                                        .hint_text("Например: 'ожидаю цикл по байтам с xor 0x42 и сравнение через memcmp в конце'")
                                        .show(ui).response;
                                    if resp.changed() {
                                        app.progress.challenge_bets.insert(ch.id.clone(), bet.clone());
                                    }
                                    if has_bet && solved && ui.small_button("✔ Сверить: гипотеза верна?").clicked() {
                                        app.progress.bet_results.insert(ch.id.clone(), true);
                                        app.toast("Ставка зафиксирована. Точность интуиции растёт!", ctx);
                                    }
                                    if has_bet && solved && ui.small_button("✘ Сверить: гипотеза мимо").clicked() {
                                        app.progress.bet_results.insert(ch.id.clone(), false);
                                        app.toast("Мимо — это тоже данные. Запиши в журнал, где ошиблась интуиция.", ctx);
                                    }
                                },
                            );
                        } else {
                            let ok = app.progress.bet_results.get(&ch.id).copied().unwrap_or(false);
                            ui.label(RichText::new(if ok { "🎯 Ставка: угадал механизм ✔" } else { "🎯 Ставка: мимо — выводы в журнал" })
                                .color(if ok { GOOD } else { WARN }).size(11.0));
                        }
                    }
                    if !solved {
                        ui.horizontal(|ui| {
                            ui.label("Флаг:");
                            let mut v = app.challenge_input.entry(ch.id.clone()).or_default().clone();
                            let resp = ui.add(egui::TextEdit::singleline(&mut v).hint_text("FLAG{...} или содержимое").desired_width(220.0));
                            app.challenge_input.insert(ch.id.clone(), v.clone());
                            if resp.clicked() || ui.button("Проверить").clicked() {
                                if app.submit_flag(&ch.id, &v) {
                                    app.toast("🚩 Верно! +50 XP", ctx);
                                    app.save();
                                } else {
                                    app.toast("Неверно — вернитесь к дизассемблеру", ctx);
                                }
                            }
                        });
                    }
                });
                ui.add_space(4.0);
            }
            ui.add_space(10.0);
            ui.strong("📝 Эталонные write-ups (как выглядит хорошо):");
            for e in &app.curriculum.ethalon_writeups {
                ui.collapsing(RichText::new(&e.pset).strong(), |ui| {
                    ui.label(&e.lesson);
                });
            }
        });
    });
}
