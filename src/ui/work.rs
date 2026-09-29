use eframe::egui::{self, RichText};

use super::{ACCENT, GOOD, WARN};
use crate::state::AppState;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = &ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        ui.heading(RichText::new("💼 Рабочая сессия аналитика").color(ACCENT));
        ui.label("Симуляция реального процесса: приходит тикет с бинарём — ты ведёшь его от триажа до отчёта. Приложение следит за МЕТОДОЛОГИЕЙ (порядок этапов), временем и полнотой отчёта. Руки работают в настоящих Ghidra/x64dbg на экспортированных файлах — здесь живёт процесс.");
        ui.add_space(8.0);

        if !app.work.active {
            // Сводка истории
            if !app.work.history.is_empty() {
                let n = app.work.history.len();
                let avg: u64 = app.work.history.iter().map(|h| h.1).sum::<u64>() / n as u64;
                let method_ok = app.work.history.iter().filter(|h| h.2).count();
                let avg_comp: u32 = app.work.history.iter().map(|h| h.3 as u32).sum::<u32>() / n as u32;
                ui.group(|ui| {
                    ui.label(RichText::new("📈 Ваша практика").strong());
                    ui.label(format!("Тикетов закрыто: {n}"));
                    ui.label(format!("Среднее время: {avg} сек ({:.1} мин)", avg as f32 / 60.0));
                    ui.label(format!("Методология без нарушений: {method_ok}/{n}"));
                    ui.label(format!("Средняя полнота отчёта: {avg_comp}%"));
                });
                ui.add_space(8.0);
            }
            if ui.add(egui::Button::new(RichText::new("📥 Получить новый тикет").color(ACCENT).size(16.0))).clicked() {
                app.start_work_session();
            }
            ui.label(RichText::new("Тикет = случайный нерешённый челлендж с легендой «файл с хоста».").weak().size(12.0));
            return;
        }

        // Легенда тикета
        let id = app.work.challenge_id.clone();
        let ch = app.curriculum.challenges.iter().find(|c| c.id == id).cloned();
        egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(40, 32, 30)).show(ui, |ui| {
            ui.label(RichText::new("🚨 ТИКЕТ #INC-2026 / Входящее подозрение").color(WARN).strong());
            ui.label(format!("Файл: {id} (уровень {})", ch.as_ref().map(|c| c.level).unwrap_or(0)));
            ui.label("Легенда: с хоста пользователя снят подозрительный исполняемый файл. Требуется определить его назначение и составить отчёт.");
            if ui.small_button("📤 Экспортировать файлы в лабу").clicked() {
                match app.export_challenge(&id) {
                    Some(dir) => app.toast(format!("Файлы в {dir} — работай в Ghidra/x64dbg"), ctx),
                    None => app.toast("Не удалось экспортировать", ctx),
                }
            }
        });
        ui.add_space(8.0);

        // Таймер
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let elapsed = now.saturating_sub(app.work.started_unix);
        ui.label(RichText::new(format!("⏱ Прошло: {:02}:{:02}", elapsed / 60, elapsed % 60)).strong());

        // Этапы
        ui.add_space(6.0);
        ui.label(RichText::new("Этапы (отмечай по мере выполнения):").strong());
        for (key, label) in [("triage", "1. Триаж без запуска (file/DIE/FLOSS)"),
                              ("static", "2. Статика (Ghidra/IDA: декомпиляция, xrefs)"),
                              ("dynamic", "3. Динамика (отладчик/Frida для проверки гипотез)"),
                              ("report", "4. Отчёт по rubric")] {
            let mut done = app.work.stages_done.iter().any(|s| s == key);
            if ui.checkbox(&mut done, label).changed() {
                if done {
                    app.work.stages_done.push(key.to_string());
                    if let Err(e) = app.work.methodology_ok() {
                        app.toast(format!("⚠ {e}"), ctx);
                    }
                } else {
                    app.work.stages_done.retain(|s| s != key);
                }
            }
        }

        // Замечание методологии
        if let Err(e) = app.work.methodology_ok() {
            ui.label(RichText::new(format!("🚨 {e}")).color(ACCENT).size(12.0));
        }

        ui.add_space(8.0);
        ui.separator();

        // Гипотезы
        ui.label(RichText::new("🔬 Гипотезы (записывай ДО проверки):").strong());
        ui.horizontal(|ui| {
            let mut h = app.work_hypothesis_input.clone();
            let resp = egui::TextEdit::singleline(&mut h)
                .desired_width(420.0)
                .hint_text("напр. 'проверка пароля через memcmp после xor 0x42'")
                .show(ui).response;
            if resp.changed() {
                app.work_hypothesis_input = h;
            }
            if ui.button("➕").clicked() && !app.work_hypothesis_input.trim().is_empty() {
                app.work.hypotheses.push(app.work_hypothesis_input.clone());
                app.work_hypothesis_input.clear();
            }
        });
        for (i, h) in app.work.hypotheses.iter().enumerate() {
            ui.label(RichText::new(format!("{}. {}", i + 1, h)).size(12.0));
        }

        ui.add_space(6.0);
        // Заметки этапов
        egui::CollapsingHeader::new("📝 Заметки триажа / статики / динамики")
            .id_salt("work_notes")
            .show(ui, |ui| {
                ui.label("Триаж:");
                ui.add(egui::TextEdit::multiline(&mut app.work.triage_notes).desired_rows(2));
                ui.label("Статика:");
                ui.add(egui::TextEdit::multiline(&mut app.work.static_notes).desired_rows(2));
                ui.label("Динамика:");
                ui.add(egui::TextEdit::multiline(&mut app.work.dynamic_notes).desired_rows(2));
            });

        // Флаг
        ui.add_space(6.0);
        ui.checkbox(&mut app.work.flag_found, "🏁 Флаг найден и проверен в приложении");

        ui.add_space(8.0);
        ui.separator();
        // Отчёт
        ui.label(RichText::new(format!("📋 Отчёт по rubric (полнота: {}%)", app.work.report_completeness())).strong());
        for (i, field) in app.work.report.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", i + 1)).weak());
                ui.add(egui::TextEdit::singleline(field).desired_width(520.0).hint_text(RUBRIC_HINTS[i]));
            });
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let can_finish = app.work.report_completeness() >= 30;
            if ui.add_enabled(can_finish, egui::Button::new(RichText::new("✅ Закрыть тикет").color(GOOD))).clicked() {
                let msg = app.finish_work_session();
                app.toast(msg, ctx);
            }
            if !can_finish {
                ui.label(RichText::new("Заполни хотя бы 3 пункта отчёта, чтобы закрыть тикет.").weak().size(12.0));
            }
            if ui.button("✖ Прервать сессию").clicked() {
                app.work.active = false;
            }
        });
    });
}

const RUBRIC_HINTS: [&str; 10] = [
    "1. Идентификация (что за файл, формат, архитектура)",
    "2. Хеши (SHA256, размер)",
    "3. Триаж-вывод (упаковщик, компилятор, энтропия)",
    "4. Строки/IOC, если есть",
    "5. Статический разбор ключевых функций",
    "6. Динамические подтверждения (что наблюдал)",
    "7. Назначение/поведение (гипотеза о цели)",
    "8. Техники/ATT&CK, если применимо",
    "9. Уверенность и что НЕ установлено (честно)",
    "10. Выводы и рекомендации",
];
