use super::theme::{accent, good, soft, warn};
use super::widgets::tinted;
use crate::rng::time_seed;
use crate::state::{AppState, WorkSession, WORK_MIN_COMPLETENESS};
use crate::util;
use eframe::egui::{self, RichText};
use std::time::Duration;

pub(super) fn show(app: &mut AppState, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::CentralPanel::default().show(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(RichText::new("💼 Рабочая сессия аналитика").color(accent()));
            ui.label("Симуляция реального процесса: приходит тикет с бинарём — ты ведёшь его от триажа до отчёта. Время, порядок этапов и полнота отчёта фиксируются.");
            ui.add_space(8.0);
            if app.work.active {
                ticket(app, ui, &ctx);
            } else {
                lobby(app, ui);
            }
        });
    });
}

fn lobby(app: &mut AppState, ui: &mut egui::Ui) {
    let history = &app.progress.work_history;
    if !history.is_empty() {
        let n = history.len() as u64;
        let avg = history.iter().map(|h| h.seconds).sum::<u64>() / n;
        let method_ok = history.iter().filter(|h| h.method_ok).count();
        let avg_completeness = history
            .iter()
            .map(|h| u64::from(h.completeness))
            .sum::<u64>()
            / n;
        ui.group(|ui| {
            ui.label(RichText::new("📈 Ваша практика").strong());
            ui.label(format!("Тикетов закрыто: {n}"));
            ui.label(format!(
                "Среднее время: {avg} сек ({:.1} мин)",
                avg as f32 / 60.0
            ));
            ui.label(format!("Методология без нарушений: {method_ok}/{n}"));
            ui.label(format!("Средняя полнота отчёта: {avg_completeness}%"));
        });
        ui.add_space(8.0);
    }
    if ui
        .add(egui::Button::new(
            RichText::new("📥 Получить новый тикет")
                .color(accent())
                .size(16.0),
        ))
        .clicked()
    {
        app.start_work_session(util::unix_now(), time_seed());
    }
    ui.label(
        RichText::new("Тикет = случайный нерешённый челлендж с легендой «файл с хоста».")
            .weak()
            .size(12.0),
    );
}

fn ticket(app: &mut AppState, ui: &mut egui::Ui, ctx: &egui::Context) {
    let id = app.work.challenge_id.clone();
    let level = app
        .curriculum
        .challenges
        .iter()
        .find(|c| c.id == id)
        .map_or(0, |c| c.level);
    tinted(ui, soft(40, 32, 30), |ui| {
        ui.label(
            RichText::new("🚨 ТИКЕТ #INC-2026 / Входящее подозрение")
                .color(warn())
                .strong(),
        );
        ui.label(format!("Файл: {id} (уровень {level})"));
        ui.label("Легенда: с хоста пользователя снят подозрительный исполняемый файл. Требуется определить, что он делает, и оформить отчёт для SOC.");
        if ui.small_button("📤 Экспортировать файлы в лабу").clicked() {
            match app.export_challenge(&id) {
                Ok(dir) => app.toast(format!("Файлы в {dir} — работай в Ghidra/x64dbg"), ctx),
                Err(e) => app.toast(format!("Не удалось экспортировать: {e}"), ctx),
            }
        }
    });
    ui.add_space(8.0);

    let elapsed = util::unix_now().saturating_sub(app.work.started_unix);
    ui.label(RichText::new(format!("⏱ Прошло: {:02}:{:02}", elapsed / 60, elapsed % 60)).strong());
    ctx.request_repaint_after(Duration::from_secs(1));

    ui.add_space(6.0);
    ui.label(RichText::new("Этапы (отмечай по мере выполнения):").strong());
    for (key, label) in [
        ("triage", "1. Триаж без запуска (file/DIE/FLOSS)"),
        ("static", "2. Статика (Ghidra/IDA: декомпиляция, xrefs)"),
        (
            "dynamic",
            "3. Динамика (отладчик/Frida для проверки гипотез)",
        ),
        ("report", "4. Отчёт по rubric"),
    ] {
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
    if let Err(e) = app.work.methodology_ok() {
        ui.label(RichText::new(format!("🚨 {e}")).color(accent()).size(12.0));
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("🔬 Гипотезы (записывай ДО проверки, минимум одна):").strong());
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.work_hypothesis_input)
                .desired_width(420.0)
                .hint_text("напр. «проверка пароля через memcmp после xor 0x42»"),
        );
        if ui.button("➕").clicked() && !app.work_hypothesis_input.trim().is_empty() {
            let hypothesis = std::mem::take(&mut app.work_hypothesis_input);
            app.work.hypotheses.push(hypothesis);
        }
    });
    for (i, h) in app.work.hypotheses.iter().enumerate() {
        ui.label(RichText::new(format!("{}. {h}", i + 1)).size(12.0));
    }

    ui.add_space(6.0);
    egui::CollapsingHeader::new("📝 Заметки триажа / статики / динамики")
        .id_salt("work_notes")
        .show(ui, |ui| {
            for (label, notes) in [
                ("Триаж:", &mut app.work.triage_notes),
                ("Статика:", &mut app.work.static_notes),
                ("Динамика:", &mut app.work.dynamic_notes),
            ] {
                ui.label(label);
                ui.add(
                    egui::TextEdit::multiline(notes)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                );
            }
        });
    ui.add_space(6.0);
    ui.checkbox(
        &mut app.work.flag_found,
        "🏁 Флаг найден и проверен в приложении",
    );

    ui.add_space(8.0);
    ui.separator();
    ui.label(
        RichText::new(format!(
            "📋 Отчёт по rubric (полнота: {}%)",
            app.work.report_completeness()
        ))
        .strong(),
    );
    for (i, field) in app.work.report.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{}.", i + 1)).weak());
            ui.add(
                egui::TextEdit::singleline(field)
                    .desired_width(520.0)
                    .hint_text(RUBRIC_HINTS[i]),
            );
        });
    }

    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        let can_finish = app.work_can_finish();
        if ui
            .add_enabled(
                can_finish,
                egui::Button::new(RichText::new("✅ Закрыть тикет").color(good())),
            )
            .clicked()
        {
            let message = app.finish_work_session(util::unix_now());
            app.toast_for(message, ctx, 8.0);
        }
        if !can_finish {
            let needed = WORK_MIN_COMPLETENESS / 10;
            ui.label(
                RichText::new(format!(
                    "Нужны этапы «триаж» и «статика», хотя бы одна гипотеза и {needed} заполненных пункта отчёта."
                ))
                .weak()
                .size(12.0),
            );
        }
        if ui.button("✖ Прервать сессию").clicked() {
            app.work = WorkSession::new();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_report_point_has_a_hint() {
        let s = WorkSession::new();
        assert_eq!(RUBRIC_HINTS.len(), s.report.len());
    }
}
