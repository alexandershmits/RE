use eframe::egui::{self, Color32, RichText, ScrollArea};

use crate::state::{AppState, QuizSession, Tab};

const ACCENT: Color32 = Color32::from_rgb(220, 60, 70); // 🩸 blood red
const GOOD: Color32 = Color32::from_rgb(90, 200, 120);
const WARN: Color32 = Color32::from_rgb(240, 180, 70);

pub fn run(app: &mut AppState, ctx: &egui::Context) {
    // toast
    if let Some((_msg, until)) = app.toast.clone() {
        if app.now(ctx) > until {
            app.toast = None;
        } else if let Some((m, _)) = &app.toast {
            egui::Area::new(egui::Id::new("toast"))
                .anchor(egui::Align2::CENTER_TOP, [0.0, 12.0])
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(Color32::from_rgb(40, 44, 56))
                        .stroke(egui::Stroke::new(1.0_f32, ACCENT))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.colored_label(GOOD, format!("✔ {m}"));
                        });
                });
        }
    }

    // PSet self-explain modal
    if let Some(key) = app.pset_pending_explain.clone() {
        egui::Window::new("📝 Правило честности курса")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("Объясните решение своими словами (2–3 предложения). Не можете объяснить — значит, пока не решили. Это защита от самообмана.").weak());
                ui.add_space(6.0);
                let mut text = app
                    .progress
                    .pset_explains
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();
                ui.add_sized([420.0, 100.0], egui::TextEdit::multiline(&mut text));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("✔ Сдать").strong()).clicked() {
                        let k = key.clone();
                        app.pset_pending_explain = None;
                        app.complete_pset_with_explain(&k, text.clone());
                        app.save();
                    }
                    if ui.button("Отмена").clicked() {
                        app.pset_pending_explain = None;
                    }
                });
            });
    }

    // achievement popups
    if !app.new_achievements.is_empty() {
        let name = app.new_achievements[0].clone();
        egui::Area::new(egui::Id::new("ach_popup"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(Color32::from_rgb(46, 38, 20))
                    .stroke(egui::Stroke::new(1.5_f32, WARN))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.strong(RichText::new("🏅 НОВАЯ АЧИВКА!").color(WARN).size(16.0));
                            ui.label(&name);
                            if ui.button("Отлично!").clicked() {
                                app.new_achievements.remove(0);
                            }
                        });
                    });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    egui::TopBottomPanel::top("topbar").show(ctx, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("🩸 RE-50").color(ACCENT).size(22.0));
            ui.label(RichText::new(&app.curriculum.course.subtitle).weak().size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("⭐ {} XP", app.progress.xp)).color(WARN));
                let pct = app.overall_percent();
                ui.add(
                    egui::ProgressBar::new(pct / 100.0)
                        .text(format!("{:.0}% курса", pct))
                        .desired_width(160.0),
                );
            });
        });
        ui.add_space(4.0);
        ui.separator();
    });

    egui::SidePanel::left("nav").exact_width(190.0).show(ctx, |ui| {
        ui.add_space(8.0);
        for (tab, icon, label) in [
            (Tab::Dashboard, "🏠", "Дашборд"),
            (Tab::Course, "📚", "Курс"),
            (Tab::Trainer, "🎯", "Тренажёр"),
            (Tab::Achievements, "🏅", "Ачивки"),
            (Tab::Resources, "🔗", "Ресурсы"),
            (Tab::Journal, "📓", "Журнал"),
            (Tab::Cards, "🃏", "Карточки"),
            (Tab::Interview, "🎤", "Интервью"),
            (Tab::Rubric, "📋", "Rubric отчёта"),
            (Tab::Diagrams, "🗺", "Схемы"),
            (Tab::Placement, "🧪", "Тест входа"),
            (Tab::Sims, "⚙️", "Симуляторы"),
            (Tab::Drills, "🔁", "Дриллы (89)"),
            (Tab::Challenges, "🚩", "Челленджи (15)"),
            (Tab::Reexam, "🎓", "Re-certification"),
            (Tab::Opponent, "🥋", "Оппонент"),
        ] {
            let selected = app.tab == tab;
            if ui
                .selectable_label(selected, format!("{icon}  {label}"))
                .clicked()
            {
                app.tab = tab;
                if app.tab == Tab::Trainer {
                    app.quiz = None;
                }
            }
        }
        ui.separator();
        ui.label(RichText::new("Правила курса:").weak());
        ScrollArea::vertical().show(ui, |ui| {
            for r in &app.curriculum.course.rules {
                ui.label(RichText::new(format!("• {r}")).size(11.0).weak());
            }
        });
    });

    let tab = std::mem::take(&mut app.tab);
    match tab {
        Tab::Dashboard => dashboard(app, ctx),
        Tab::Course => course(app, ctx),
        Tab::Trainer => trainer(app, ctx),
        Tab::Achievements => achievements(app, ctx),
        Tab::Resources => resources(app, ctx),
        Tab::Journal => journal(app, ctx),
        Tab::Cards => cards(app, ctx),
        Tab::Interview => interview(app, ctx),
        Tab::Rubric => rubric(app, ctx),
        Tab::Diagrams => diagrams(app, ctx),
        Tab::Placement => placement(app, ctx),
        Tab::Sims => sims(app, ctx),
        Tab::Drills => drills(app, ctx),
        Tab::Challenges => challenges(app, ctx),
        Tab::Reexam => reexam(app, ctx),
        Tab::Opponent => opponent_tab(app, ctx),
    }
    app.tab = tab;
}

fn dashboard(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            // Внезапный экзамен: баннер, когда срок подошёл
            {
                let day = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() / 86400)
                    .unwrap_or(0);
                if app.reexam_due(day) {
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(46, 38, 20))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🎓 Пора на re-certification!").color(WARN).size(15.0));
                                if ui.small_button("Начать").clicked() {
                                    let seed = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(1);
                                    app.start_reexam(seed);
                                    app.tab = Tab::Reexam;
                                }
                            });
                        });
                    ui.add_space(6.0);
                }
            }
            ui.heading(RichText::new(&app.curriculum.course.title).color(ACCENT).size(26.0));

            // Поведенческий анализ (анти-паттерны)
            // Точность интуиции (ставки)
            if !app.progress.bet_results.is_empty() {
                let total = app.progress.bet_results.len();
                let hit = app.progress.bet_results.values().filter(|b| **b).count();
                let pct = 100.0 * hit as f32 / total as f32;
                ui.add_space(6.0);
                ui.label(RichText::new(format!(
                    "🎯 Точность гипотез: {hit}/{total} ({pct:.0}%) — {}",
                    if pct >= 60.0 { "интуиция калибруется" } else { "пока угадывание — копай глубже перед ставкой" }
                )).size(13.0));
            }
            {
                let findings = crate::detector::analyze(&app.progress, &app.curriculum);
                if !findings.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("🔎 Поведенческий анализ").strong().size(15.0));
                    for f in &findings {
                        egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(40, 40, 30)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} {}", f.severity.icon(), f.title))
                                    .color(f.severity.color32()).strong());
                            });
                            ui.label(RichText::new(&f.advice).size(13.0));
                        });
                        ui.add_space(4.0);
                    }
                }
            }
            ui.label(&app.curriculum.course.subtitle);
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                for (big, small, color) in [
                    (
                        format!("{}/{}", app.weeks_done_count(), app.weeks_total()),
                        "недель закрыто",
                        ACCENT,
                    ),
                    (
                        format!("{}/{}", app.psets_done_count(), app.psets_total()),
                        "PSets сдано",
                        WARN,
                    ),
                    (
                        format!(
                            "{}/{}",
                            app.progress.quiz_correct.len(),
                            app.curriculum.quizzes.len()
                        ),
                        "квизов верно",
                        GOOD,
                    ),
                    (
                        format!(
                            "{}/{}",
                            app.progress.achievements.len(),
                            app.curriculum.achievements.len()
                        ),
                        "ачивок получено",
                        Color32::from_rgb(200, 140, 255),
                    ),
                ] {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new(big).color(color).size(24.0));
                        ui.label(RichText::new(small).weak().size(11.0));
                    });
                }
            });
            ui.add_space(12.0);

            ui.heading("📌 Следующая неделя");
            let next = app
                .curriculum
                .weeks
                .iter()
                .position(|w| !app.progress.weeks_done.contains(&w.id))
                .unwrap_or(app.curriculum.weeks.len().saturating_sub(1));
            let w = &app.curriculum.weeks[next];
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(w.label());
                    ui.label(&w.title);
                    ui.label(RichText::new(app.curriculum.module_name(w.module)).weak());
                    if ui.button("Открыть →").clicked() {
                        app.selected_week = next;
                        app.tab = Tab::Course;
                    }
                });
            });
            ui.add_space(12.0);

            ui.heading("🔥 Активность");
            let (last_day, streak) = app.progress.streak;
            let today = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            let active_today = last_day == today;
            ui.label(RichText::new(format!(
                "Серия: {} дн. {}",
                streak,
                if active_today { "(сегодня отмечено ✓)" } else { "(зайдите сегодня!)" }
            )).size(14.0).color(if active_today { GOOD } else { WARN }));

            // XP sparkline (simple bars, last 30 days)
            let h = &app.progress.xp_history;
            if h.len() >= 2 {
                ui.add_space(4.0);
                ui.label(RichText::new("XP за последние дни:").weak().size(11.0));
                let recent: Vec<(u64, u32)> = h[h.len().saturating_sub(30)..].to_vec();
                let min = recent.iter().map(|x| x.1).min().unwrap_or(0);
                let max = recent.iter().map(|x| x.1).max().unwrap_or(1);
                let range = (max - min).max(1) as f32;
                ui.horizontal_wrapped(|ui| {
                    for (_day, xp) in &recent {
                        let frac = (*xp - min) as f32 / range;
                        let hgt = 4.0 + frac * 28.0;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(6.0, 32.0),
                            egui::Sense::hover(),
                        );
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - hgt),
                            egui::pos2(rect.right(), rect.bottom()),
                        );
                        ui.painter().rect_filled(bar, 1.0, ACCENT.gamma_multiply(0.4 + frac * 0.6));
                    }
                });
            }
            ui.add_space(10.0);

            ui.heading("📊 Слабые темы (по неверным ответам квизов)");
            let weak = app.weak_topics();
            if weak.is_empty() {
                ui.label(RichText::new("Пока нет данных — пройдите квизы, и я покажу, что подтянуть.").weak());
            } else {
                for (name, n) in &weak {
                    ui.label(RichText::new(format!("🔴 {name}: {n} неверных — перейдите в карточки и дриллы этой темы")).size(13.0));
                }
            }
            ui.add_space(10.0);

            ui.heading("📤 Экспорт / импорт прогресса");
            ui.horizontal(|ui| {
                if ui.button("Копировать JSON в буфер").clicked() {
                    ui.ctx().copy_text(app.export_progress());
                    app.toast("Прогресс скопирован — сохраните в файл!", ctx);
                }
                if ui.button("Импортировать из буфера").clicked() {
                    let txt = ui.ctx().input(|i| i.events.iter().find_map(|e| match e {
                        egui::Event::Paste(s) => Some(s.clone()), _ => None }).unwrap_or_default());
                    if txt.is_empty() {
                        app.toast("Скопируйте JSON в буфер (Ctrl+C), затем нажмите импорт", ctx);
                    } else if app.import_progress(&txt) {
                        app.toast("Прогресс импортирован!", ctx);
                    } else {
                        app.toast("Ошибка: некорректный JSON", ctx);
                    }
                }
            });
            ui.add_space(10.0);

            ui.heading("📤 Карточка прогресса");
            ui.horizontal(|ui| {
                if ui.button("Сгенерировать карточку прогресса").clicked() {
                    let pct = app.overall_percent() as u32;
                    app.progress_export_text = format!(
                        "🩸 RE-50 | Недели: {}/{} | PSets: {}/{} | Квизы: {}/{} | Ачивки: {}/{} | XP: {} | Прогресс: {}%",
                        app.weeks_done_count(), app.weeks_total(),
                        app.psets_done_count(), app.psets_total(),
                        app.progress.quiz_correct.len(), app.curriculum.quizzes.len(),
                        app.progress.achievements.len(), app.curriculum.achievements.len(),
                        app.progress.xp, pct
                    );
                }
                if !app.progress_export_text.is_empty() {
                    ui.monospace(&app.progress_export_text);
                    if ui.button("📋").on_hover_text("Скопировать").clicked() {
                        ui.ctx().copy_text(app.progress_export_text.clone());
                        app.toast("Скопировано в буфер обмена", ctx);
                    }
                }
            });
            ui.add_space(12.0);

            ui.heading("🗺 Карта курса");
            for m in &app.curriculum.modules {
                let weeks: Vec<_> = app
                    .curriculum
                    .weeks
                    .iter()
                    .filter(|w| w.module == m.id)
                    .collect();
                let done = weeks
                    .iter()
                    .filter(|w| app.progress.weeks_done.contains(&w.id))
                    .count();
                ui.horizontal(|ui| {
                    ui.strong(format!("{} {}", m.icon, m.name));
                    ui.label(RichText::new(format!("({}/{})", done, weeks.len())).weak());
                });
                ui.add(
                    egui::ProgressBar::new(if weeks.is_empty() { 0.0 } else { done as f32 / weeks.len() as f32 })
                        .desired_height(8.0),
                );
            }
        });
    });
}

fn course(app: &mut AppState, ctx: &egui::Context) {
    egui::SidePanel::right("week_list").exact_width(240.0).show(ctx, |ui| {
        ui.heading("Недели");
        ui.separator();
        ScrollArea::vertical().id_salt("weeklist").show(ui, |ui| {
            for (i, w) in app.curriculum.weeks.iter().enumerate() {
                let done = app.progress.weeks_done.contains(&w.id);
                let mark = if done { "✅ " } else { "▫️ " };
                let label = format!("{}{} · {}", mark, w.label(), w.title);
                if ui
                    .selectable_label(app.selected_week == i, RichText::new(label).size(12.5))
                    .clicked()
                {
                    app.selected_week = i;
                }
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let Some(w) = app.curriculum.weeks.get(app.selected_week) else { return; };
        let week = w.clone();
        ScrollArea::vertical().id_salt("weekdetail").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new(week.label()).color(ACCENT));
                ui.strong(&week.title);
            });
            ui.label(RichText::new(app.curriculum.module_name(week.module)).weak());
            ui.separator();

            // 🎯 problem-first case
            if let Some(case) = &week.case {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(38, 30, 40))
                    .stroke(egui::Stroke::new(1.0_f32, WARN))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.strong(RichText::new("🎯 Проблема недели — сначала ЗАЧЕМ, потом КАК:").color(WARN));
                        ui.label(case);
                    });
                ui.add_space(8.0);
            }

            // 📖 lectures
            if !week.lectures.is_empty() {
                ui.heading("📖 Лекции");
                for l in &week.lectures {
                    ui.label(format!("• {l}"));
                }
                ui.add_space(8.0);
            }

            // 🧪 lab
            if let Some(lab) = &week.lab {
                ui.heading(format!("🧪 {}", lab.title));
                for (i, step) in lab.steps.iter().enumerate() {
                    let key = format!("{}:{}", week.id, i);
                    let mut checked = app.progress.lab_steps_done.contains(&key);
                    if ui.checkbox(&mut checked, step).changed() {
                        app.toggle_lab_step(&key);
                        app.save();
                    }
                }
                ui.add_space(8.0);
            }

            // 📝 psets
            if !week.psets.is_empty() {
                ui.heading("📝 Problem Sets");
                for (i, ps) in week.psets.iter().enumerate() {
                    let key = format!("{}:{}", week.id, i);
                    let done = app.progress.psets_done.contains(&key);
                    ui.horizontal(|ui| {
                        let label = if done { RichText::new("✔").color(GOOD).strong() } else { RichText::new("▢").weak() };
                        if ui.button(label).clicked() {
                            app.pset_pending_explain = Some(key.clone());
                        }
                        let rt = if done { RichText::new(ps).weak().strikethrough() } else { RichText::new(ps).strong() };
                        ui.label(rt);
                    });
                    if let Some(ex) = app.progress.pset_explains.get(&key) {
                        ui.label(RichText::new(format!("   ↳ объяснение: {ex}")).weak().size(11.0));
                    }
                }
                ui.add_space(8.0);
            }

            // ✅ checkpoint
            if !week.checkpoint.is_empty() {
                ui.heading("✅ Чек-пойнт: понял или имитирую?");
                for (i, c) in week.checkpoint.iter().enumerate() {
                    let mut v = app
                        .progress
                        .checkpoint
                        .get(&week.id)
                        .and_then(|v| v.get(i).copied())
                        .unwrap_or(false);
                    if ui.checkbox(&mut v, c).changed() {
                        app.toggle_checkpoint(&week.id, i);
                        app.save();
                    }
                }
                ui.add_space(8.0);
            }

            ui.separator();
            let mut done = app.progress.weeks_done.contains(&week.id);
            if ui
                .checkbox(&mut done, RichText::new("Неделя закрыта (+30 XP)").strong())
                .changed()
            {
                app.toggle_week_done(&week.id);
                app.save();
            }
        });
    });
}

fn trainer(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        if app.quiz.is_none() {
            ui.heading("🎯 Тренажёр-квиз");
            ui.label("Проверь себя: понимание или имитация? Выбери модуль и начни сессию.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("▶ Все модули").clicked() {
                    app.quiz = Some(QuizSession::new(app.curriculum.quizzes.len(), None));
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for m in &app.curriculum.modules {
                    let n = app
                        .curriculum
                        .quizzes
                        .iter()
                        .filter(|q| q.module == m.id)
                        .count();
                    if n > 0 && ui.button(format!("{} ({n})", m.name)).clicked() {
                        app.quiz = Some(QuizSession::new(app.curriculum.quizzes.len(), Some(m.id)));
                    }
                }
            });
            ui.add_space(16.0);
            ui.separator();
            ui.label(RichText::new("Прогресс по вопросам:").weak());
            for m in &app.curriculum.modules {
                let total = app.curriculum.quizzes.iter().filter(|q| q.module == m.id).count();
                let ok = app
                    .curriculum
                    .quizzes
                    .iter()
                    .filter(|q| q.module == m.id)
                    .filter(|q| app.progress.quiz_correct.contains(&q.id))
                    .count();
                if total > 0 {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}:", m.icon, m.name));
                        ui.add(
                            egui::ProgressBar::new(ok as f32 / total as f32)
                                .text(format!("{ok}/{total}"))
                                .desired_width(260.0),
                        );
                    });
                }
            }
            return;
        }

        // take quiz session out to avoid borrow conflicts
        let mut q = app.quiz.take().unwrap();
        let pos = q.pos.min(q.order.len() - 1);
        let quiz_idx = q.order[pos];
        let quiz = app.curriculum.quizzes[quiz_idx].clone();

        let mut finish = false;
        {
            ui.horizontal(|ui| {
                ui.heading(format!("Вопрос {}/{}", q.pos + 1, q.order.len()));
                if let Some(m) = q.filter_module {
                    ui.label(RichText::new(app.curriculum.module_name(m)).weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Завершить сессию").clicked() {
                        finish = false;
                        q.pos = q.order.len(); // mark aborted
                    }
                });
            });
            ui.add(
                egui::ProgressBar::new((q.pos as f32) / q.order.len() as f32)
                    .desired_height(8.0),
            );
            ui.add_space(12.0);

            ui.strong(RichText::new(&quiz.question).size(17.0));
            ui.add_space(6.0);

            let mut click_answer: Option<usize> = None;
            for (i, a) in quiz.answers.iter().enumerate() {
                let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
                if q.submitted {
                    if i == quiz.correct {
                        text = text.color(GOOD).strong();
                    } else if Some(i) == q.selected {
                        text = text.color(ACCENT).strong();
                    }
                }
                let enabled = !q.submitted;
                let resp = ui.add_enabled(
                    enabled,
                    egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap).min_size(egui::vec2(0.0, 30.0)),
                );
                if resp.clicked() {
                    click_answer = Some(i);
                }
            }

            if let Some(sel) = click_answer {
                q.selected = Some(sel);
                q.submitted = true;
                q.answered += 1;
                if app.submit_quiz_answer(&quiz, sel) {
                    q.correct_count += 1;
                }
            }

            if q.submitted {
                let ok = q.selected == Some(quiz.correct);
                ui.add_space(8.0);
                if ok {
                    ui.label(RichText::new("✔ Верно!").color(GOOD).size(16.0).strong());
                } else {
                    ui.label(
                        RichText::new(format!(
                            "✘ Неверно. Правильный ответ: {}",
                            char::from(b'A' + quiz.correct as u8)
                        ))
                        .color(ACCENT)
                        .size(16.0)
                        .strong(),
                    );
                ui.label(RichText::new("💡 Тема уйдёт в карточки: «Повторить» понижает уровень карточки и поднимает её в очереди.").weak().size(11.0));
                }
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(format!("💡 {}", quiz.explain)));
                });
                ui.add_space(8.0);
                if ui.button("Далее →").clicked() {
                    q.pos += 1;
                    q.selected = None;
                    q.submitted = false;
                    if q.pos >= q.order.len() {
                        q.finished = true;
                    }
                }
            }
        } // ui borrow ends

        app.save();
        if q.finished {
            let pct = (q.correct_count as f32 / q.answered.max(1) as f32 * 100.0) as u32;
            app.toast(format!("Сессия завершена: {}/{} ({pct}%)", q.correct_count, q.answered), ctx);
        } else {
            app.quiz = Some(q);
        }
    });
}

fn achievements(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("🏅 Ачивки");
            ui.label(RichText::new(format!(
                "Получено {} из {} · {} XP",
                app.progress.achievements.len(),
                app.curriculum.achievements.len(),
                app.progress.xp
            )));
            ui.separator();
            egui::Grid::new("achgrid").num_columns(3).spacing([16.0, 12.0]).show(ui, |ui| {
                for a in &app.curriculum.achievements {
                    let got = app.progress.achievements.contains(&a.id);
                    ui.vertical(|ui| {
                        let name = if got {
                            RichText::new(&a.name).size(18.0).strong()
                        } else {
                            RichText::new("🔒 ? ? ?").weak().size(18.0)
                        };
                        ui.label(name);
                        if got {
                            ui.label(RichText::new(&a.desc).size(11.5));
                            ui.label(RichText::new(format!("+{} XP", a.xp)).color(WARN).size(11.0));
                        } else {
                            ui.label(RichText::new("Условие скрыто — иди учись :)").weak().size(11.0));
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });
}

fn resources(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            ui.heading("🔗 Библиотека ресурсов");
            ui.label(RichText::new("Все инструменты и материалы курса — бесплатные или с free-версией.").weak());
            ui.separator();
            let mut last_cat = String::new();
            for r in &app.curriculum.resources {
                if r.category != last_cat {
                    ui.add_space(6.0);
                    ui.strong(RichText::new(&r.category).color(ACCENT));
                    last_cat = r.category.clone();
                }
                ui.horizontal(|ui| {
                    if ui.link(&r.name).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(&r.url));
                    }
                    ui.label(RichText::new(&r.url).weak().size(10.5));
                });
            }
        });
    });
}

fn journal(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("📓 Журнал");
        ui.label(RichText::new(
            "Каждый разобранный бинарь: скриншоты, псевдокод, что понял / что не понял. \
             Write-up каждого PSet — по правилам честности курса.",
        ).weak());
        ui.separator();
        let mut text = app.progress.journal.clone();
        let response = ScrollArea::vertical()
            .id_salt("journal")
            .show(ui, |ui| {
                ui.add_sized(
                    [ui.available_width(), 500.0],
                    egui::TextEdit::multiline(&mut text).desired_rows(24).code_editor(),
                )
            });
        if response.inner.changed() {
            app.progress.journal = text.clone();
        }
        // save button below also persists
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Символов: {}", app.progress.journal.len())).weak().size(11.0));
            if ui.button("💾 Сохранить").clicked() {
                app.progress.journal = text;
                app.save();
                app.toast("Журнал сохранён", ctx);
            }
        });
    });
}


fn cards(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🃏 Карточки (spaced repetition)");
        ui.label(RichText::new("Уровень карточки растёт при ответе «знаю» и падает при «повторить». Интервалы: 1д → 3д → 7д → 21д → 60д.").weak());
        ui.separator();

        let total = app.curriculum.flashcards.len();
        if app.cards.is_none() {
            if total == 0 { return; }
            ui.label(format!("Колода: {total} карточек"));
            // box stats
            let mut levels = [0usize; 6];
            for c in &app.curriculum.flashcards {
                let l = app.progress.card_levels.get(&c.id).copied().unwrap_or(0) as usize;
                levels[l.min(5)] += 1;
            }
            ui.horizontal(|ui| {
                for (i, n) in levels.iter().enumerate() {
                    ui.label(format!("L{i}: {n}"));
                }
            });
            if ui.button("▶ Начать сессию (слабые — первыми)").clicked() {
                // order: lowest level first, shuffled within level
                let mut idx: Vec<usize> = (0..total).collect();
                idx.sort_by_key(|&i| app.progress.card_levels.get(&app.curriculum.flashcards[i].id).copied().unwrap_or(0));
                app.cards = Some(crate::state::CardSession::new(0)); // placeholder then override
                if let Some(cs) = app.cards.as_mut() {
                    cs.order = idx;
                    cs.pos = 0;
                }
            }
            return;
        }

        let Some(cs) = app.cards.as_mut() else { return; };
        if cs.pos >= cs.order.len() {
            // session end
            let known = cs.known.len();
            let again = cs.again.len();
            ui.heading("Сессия завершена!");
            ui.label(format!("Знаю: {known} · Повторить: {again}"));
            if ui.button("Закончить").clicked() { app.cards = None; }
            app.save();
            return;
        }
        let card = app.curriculum.flashcards[cs.order[cs.pos]].clone();
        ui.label(RichText::new(format!("{}/{}", cs.pos + 1, cs.order.len())).weak());
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(ui.available_width() - 16.0);
            ui.strong(RichText::new(&card.front).size(17.0));
            if cs.show_back {
                ui.separator();
                ui.label(RichText::new(&card.back).size(15.0));
                ui.label(RichText::new(format!("теги: {}", card.tags.join(", "))).weak().size(10.0));
            }
        });
        ui.add_space(8.0);
        if !cs.show_back {
            if ui.button("Показать ответ").clicked() { cs.show_back = true; }
        } else {
            ui.horizontal(|ui| {
                if ui.button("🔁 Повторить").clicked() {
                    cs.again.push(cs.order[cs.pos]);
                    let lvl = app.progress.card_levels.get(&card.id).copied().unwrap_or(0).saturating_sub(1);
                    app.progress.card_levels.insert(card.id.clone(), lvl);
                    cs.pos += 1; cs.show_back = false;
                }
                if ui.button("✅ Знаю").clicked() {
                    cs.known.push(cs.order[cs.pos]);
                    let lvl = (app.progress.card_levels.get(&card.id).copied().unwrap_or(0) + 1).min(5);
                    app.progress.card_levels.insert(card.id.clone(), lvl);
                    cs.pos += 1; cs.show_back = false;
                }
            });
        }
    });
}

fn interview(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🎤 Банк вопросов интервью RE");
        ui.label(RichText::new("Self-interview: ответьте вслух, потом откройте эталон. Слабые вопросы — в карточки.").weak());
        ui.separator();
        ScrollArea::vertical().id_salt("interview").show(ui, |ui| {
            for q in app.curriculum.interview_questions.iter() {
                ui.collapsing(RichText::new(format!("{} · {}", q.cat, q.q)).strong(), |ui| {
                    ui.label(&q.a);
                });
            }
        });
    });
}

fn rubric(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("📋 Rubric: чек перед публикацией write-up");
        ui.label(RichText::new("10/10 пунктов = отчёт уровня сеньора. Сверяйте КАЖДЫЙ отчёт.").weak());
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| {
            let rubric_items = app.curriculum.rubric.clone();
            for (i, item) in rubric_items.iter().enumerate() {
                let key = format!("rubric:{}", i);
                let mut done = app.progress.lab_steps_done.contains(&key);
                if ui.checkbox(&mut done, format!("{}. {item}", i + 1)).changed() {
                    app.toggle_lab_step(&key);
                    app.save();
                }
            }
            let n = app.curriculum.rubric.len();
            let done = (0..n).filter(|i| app.progress.lab_steps_done.contains(&format!("rubric:{i}"))).count();
            ui.add_space(8.0);
            ui.add(egui::ProgressBar::new(done as f32 / n.max(1) as f32).text(format!("{done}/{n}")));
            if ui.button("Сбросить").clicked() {
                for i in 0..n { app.progress.lab_steps_done.remove(&format!("rubric:{i}")); }
                app.save();
            }
        });
    });
}


fn diagrams(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🗺 Визуальные схемы");
        // Topic graph at top
        if !app.curriculum.topic_map.is_empty() {
            ui.collapsing(RichText::new("🌍 КАРТА КУРСА — как связаны все темы").strong().color(ACCENT).size(15.0), |ui| {
                let mut code = app.curriculum.topic_map.clone();
                egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut code)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(42)
                            .desired_width(760.0)
                            .interactive(false),
                    );
                });
            });
            ui.add_space(6.0);
        }
        ui.label(RichText::new("Моноширинные схемы-шпаргалки: стек, vtable, PE, GOT/PLT, hollowing, пайплайн. Вернитесь к ним, когда тема встретится в практике.").weak());
        ui.separator();
        ScrollArea::vertical().id_salt("diagrams").show(ui, |ui| {
            let ds = app.curriculum.diagrams.clone();
            for d in ds {
                ui.collapsing(RichText::new(format!("📐 {} [{}]", d.title, d.tag)).strong(), |ui| {
                    let mut code = d.code.clone();
                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(12, 13, 18))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut code)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(d.code.lines().count() + 1)
                                    .desired_width(720.0)
                                    .interactive(false),
                            );
                        });
                });
            }
        });
    });
}

fn placement(app: &mut AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🧪 Placement: с какой недели вам стартовать?");
        ui.label(RichText::new("20 вопросов по базам. Не угадывайте — цель найти правильную ТОЧКУ ВХОДА, а не набить балл.").weak());
        ui.separator();

        let total = app.curriculum.placement.len();
        if total == 0 { return; }

        // track answers in a local session stored in app.quiz slot? No — dedicated state
        if app.placement.is_none() {
            if ui.button("▶ Начать тест").clicked() {
                app.placement = Some(crate::state::PlacementState { pos: 0, score: 0, selected: None, answered: false });
            }
            return;
        }
        let Some(pl) = app.placement.as_mut() else { return; };
        if pl.pos >= total {
            let score = pl.score;
            let pct = score as f32 / total as f32;
            let rec = if pct < 0.35 { "Неделя 0 — с полного нуля. Всё впереди!" }
                else if pct < 0.7 { "Недели 1–2 — C с самого начала, дальше быстрее." }
                else if pct < 0.95 { "Недели 3–6 — можно пропустить C, но пройдите asm и PE честно." }
                else { "Недели 7–9 — база есть, стартуйте с инструментов. Но прогоните чек-пойнты недель 1–6 как самопроверку." };
            ui.heading(format!("Результат: {score}/{total}"));
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.strong(RichText::new(format!("Рекомендация: {rec}")).size(16.0));
            });
            if ui.button("Сбросить").clicked() { app.placement = None; }
            return;
        }

        let q = app.curriculum.placement[pl.pos].clone();
        ui.label(format!("Вопрос {}/{}", pl.pos + 1, total));
        ui.strong(RichText::new(&q.q).size(16.0));
        ui.add_space(6.0);
        let mut click: Option<usize> = None;
        for (i, a) in q.a.iter().enumerate() {
            let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
            if pl.answered {
                if i == q.correct { text = text.color(GOOD).strong(); }
                else if Some(i) == pl.selected { text = text.color(ACCENT).strong(); }
            }
            if ui.add_enabled(!pl.answered, egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap)).clicked() {
                click = Some(i);
            }
        }
        if let Some(sel) = click {
            pl.selected = Some(sel);
            pl.answered = true;
            if sel == q.correct { pl.score += 1; }
        }
        if pl.answered {
            let ok = pl.selected == Some(q.correct);
            ui.label(if ok { RichText::new("✔ Верно").color(GOOD) } else {
                RichText::new(format!("✘ Неверно. Тема относится к неделе {}", q.week)).color(ACCENT)
            });
            if ui.button("Далее →").clicked() {
                pl.pos += 1; pl.selected = None; pl.answered = false;
            }
        }
    });
}


fn sims(app: &mut AppState, ctx: &egui::Context) {
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
    if app.sim.task_idx >= tasks.len() { app.sim.task_idx = 0; }
    let task = &tasks[app.sim.task_idx];

    ui.strong(RichText::new(&task.title).size(16.0));
    ui.label(RichText::new("Введите hex-значения (без 0x) для каждого регистра ПОСЛЕ выполнения кода:").weak());
    ui.add_space(4.0);
    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
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
            let resp = ui.add(egui::TextEdit::singleline(&mut v).desired_width(120.0).font(egui::TextStyle::Monospace));
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
                vals.push((r.to_string(), u64::from_str_radix(&s, 16).unwrap_or(u64::MAX)));
            }
            submit = Some(vals);
        }
        if ui.button("➡ Следующая задача").clicked() {
            app.sim.task_idx = (app.sim.task_idx + 1) % tasks.len();
            app.sim.user_input.clear();
            app.sim.checked = false;
            app.sim.show_answer = false;
        }
        if ui.button("💡 Ответ").clicked() { app.sim.show_answer = !app.sim.show_answer; }
    });

    if let Some(vals) = submit {
        let all_ok = vals.iter().all(|(r, v)| {
            task.answers.iter().any(|(ar, av)| *ar == r && av == v)
        });
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
            ui.label(RichText::new("✘ Есть ошибки — проверьте ещё раз или посмотрите ответ.").color(ACCENT).strong());
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
    if app.sim.task_idx >= tasks.len() { app.sim.task_idx = 0; }
    let idx = app.sim.task_idx;
    let task = &tasks[idx];

    ui.strong(RichText::new(&task.title).size(16.0));
    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
        ui.monospace(&task.bytes);
    });
    ui.add_space(4.0);
    ui.strong(&task.question);
    ui.add_space(4.0);

    let mut click: Option<usize> = None;
    for (i, a) in task.answers.iter().enumerate() {
        let mut text = RichText::new(format!("{}) {}", char::from(b'A' + i as u8), a));
        if app.sim.checked {
            if i == task.correct { text = text.color(GOOD).strong(); }
            else if app.sim.choice == Some(i) { text = text.color(ACCENT).strong(); }
        }
        if ui.add_enabled(!app.sim.checked, egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap)).clicked() {
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
        if app.sim.last_ok { ui.label(RichText::new("✔ Верно!").color(GOOD).strong()); }
        else { ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong()); }
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
    if app.sim.task_idx >= tasks.len() { app.sim.task_idx = 0; }
    let idx = app.sim.task_idx;
    let task = &tasks[idx];

    ui.strong(RichText::new(format!("Трассировка упаковщика — задача {}", idx + 1)).size(16.0));
    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
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
            if i == task.correct { text = text.color(GOOD).strong(); }
            else if app.sim.choice == Some(i) { text = text.color(ACCENT).strong(); }
        }
        if ui.add_enabled(!app.sim.checked, egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap)).clicked() {
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
        if app.sim.last_ok { ui.label(RichText::new("✔ Верно!").color(GOOD).strong()); }
        else { ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong()); }
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


fn drills(app: &mut AppState, ctx: &egui::Context) {
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
        if ui.button("💡 Ответ").clicked() { app.drill.show_answer = !app.drill.show_answer; }
        ui.label(RichText::new(format!("решено в сессии: {}", app.drill.solved.len())).weak().size(11.0));
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
            if i == correct { text = text.color(GOOD).strong(); }
            else if app.drill.choice == Some(i) { text = text.color(ACCENT).strong(); }
        }
        if ui.add_enabled(!app.drill.checked, egui::Button::new(text).wrap_mode(egui::TextWrapMode::Wrap)).clicked() {
            click = Some(i);
        }
    }
    if let Some(c) = click {
        app.drill.choice = Some(c);
        app.drill.checked = true;
        let ok = c == correct;
        if ok {
            app.drill.solved.insert(format!("{id_tag}{}", app.drill.idx));
            app.add_xp(xp);
            app.toast(format!("Верно! +{xp} XP"), ui.ctx());
        }
    }
    if app.drill.checked {
        let ok = app.drill.choice == Some(correct);
        if ok { ui.label(RichText::new("✔ Верно!").color(GOOD).strong()); }
        else { ui.label(RichText::new("✘ Неверно.").color(ACCENT).strong()); }
    }
}

fn drill_asm(app: &mut AppState, ui: &mut egui::Ui) {
    let tasks = app.curriculum.drills.asm.clone();
    if tasks.is_empty() { return; }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    ui.strong(RichText::new(&t.c).size(15.0));
    ui.add_space(4.0);
    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
        for line in &t.asm { ui.monospace(line); }
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
    if tasks.is_empty() { return; }
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
    if tasks.is_empty() { return; }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(12,13,18)).show(ui, |ui| {
        for line in t.asm.lines() { ui.monospace(line); }
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
    if tasks.is_empty() { return; }
    let t = &tasks[app.drill.idx.min(tasks.len() - 1)];
    ui.strong(RichText::new(&t.task).size(15.0));
    ui.add_space(4.0);
    ui.label(RichText::new("Напишите решение в уме/в редакторе, затем откройте эталон.").weak());
    ui.horizontal(|ui| {
        if ui.button("💡 Показать эталон").clicked() { app.drill.show_answer = !app.drill.show_answer; }
        if ui.button("✔ Знаю это").clicked() {
            let id = format!("scr{}", app.drill.idx);
            if app.drill.solved.insert(id) {
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
    ui.label(RichText::new(format!("решено в сессии: {}", app.drill.solved.len())).weak().size(11.0));
}


fn challenges(app: &mut AppState, ctx: &egui::Context) {
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

fn reexam(app: &mut crate::state::AppState, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(RichText::new("🎓 Monthly Re-certification").color(ACCENT));
        ui.add_space(6.0);
        ui.label("Раз в 30 дней приложение устраивает внезапный экзамен: 10 случайных задач из пройденного материала. Порог — 70%. Провал → темы возвращаются в слабые.");
        ui.add_space(10.0);

        if let Some(score) = &app.progress.reexam_score {
            let (date, c, t) = score;
            let passed = (*c as f32) >= 0.7 * (*t as f32);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(
                    "Последний результат: {c}/{t} ({})",
                    if passed { "порог пройден ✔" } else { "ПРОВАЛ — повторите слабые темы" }
                )).color(if passed { GOOD } else { WARN }));
                ui.label(RichText::new(date).weak());
            });
            ui.add_space(6.0);
        }

        let due = {
            let day = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            app.reexam_due(day)
        };

        if app.reexam.is_none() {
            if due {
                if ui.add(egui::Button::new(RichText::new("▶ Начать экзамен (10 вопросов)").color(ACCENT))).clicked() {
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(1);
                    app.start_reexam(seed);
                }
            } else {
                ui.label(RichText::new("⏳ Экзамен пока не назначен — пройдите больше материала.").weak());
            }
            return;
        }

        let Some(rx) = app.reexam.as_mut() else { return; };
        if rx.finished {
            ui.label("Экзамен завершён. Результат сохранён.");
            if ui.button("Закрыть").clicked() {
                app.reexam = None;
            }
            return;
        }

        let total = rx.questions.len();
        ui.label(format!("Вопрос {} / {}", rx.pos + 1, total));
        let q = &rx.questions[rx.pos];
        ui.add_space(4.0);
        ui.label(RichText::new(&q.question).size(16.0));
        ui.add_space(4.0);
        for (i, a) in q.answers.iter().enumerate() {
            let is_sel = rx.selected == Some(i);
            if ui.add(egui::RadioButton::new(is_sel, a)).clicked() && !rx.answered {
                rx.selected = Some(i);
            }
        }
        ui.add_space(8.0);
        if !rx.answered {
            if ui.add(egui::Button::new("Ответить")).clicked() {
                if let Some(sel) = rx.selected {
                    rx.answered = true;
                    if sel == q.correct { rx.correct += 1; }
                }
            }
        } else {
            let sel_ok = rx.selected == Some(q.correct);
            ui.label(
                RichText::new(if sel_ok { "✔ Верно".to_string() } else {
                    format!("✘ Неверно. Правильный ответ: {}", q.answers[q.correct])
                })
                .color(if sel_ok { GOOD } else { WARN }),
            );
            ui.label(RichText::new(&q.explain).weak().size(12.0));
            ui.add_space(6.0);
            let last = rx.pos + 1 >= total;
            let date = {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs()).unwrap_or(0);
                // days since epoch -> yyyy-mm-dd approx
                let days = secs / 86400;
                let y = 1970 + days / 365;
                format!("re-exam day {days} (~{y})")
            };
            if ui.add(egui::Button::new(if last { "Завершить экзамен" } else { "Далее ▶" })).clicked() {
                if last {
                    app.finish_reexam(date);
                } else {
                    rx.pos += 1;
                    rx.selected = None;
                    rx.answered = false;
                }
            }
        }
    });
}

fn opponent_tab(app: &mut AppState, ctx: &egui::Context) {
    use crate::opponent;
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading(RichText::new("🥋 Socratic-оппонент").color(ACCENT));
        ui.label("Атакует твои объяснения: не даёт ответов — задает каверзные вопросы. Пересказ и «так принято» здесь не работают: только понимание. Лучший суррогат ментора в solo-обучении.");
        ui.add_space(10.0);

        // Режим LLM
        let llm_online = opponent::ollama_available();
        ui.horizontal(|ui| {
            if llm_online {
                ui.label(RichText::new("🟢 Ollama обнаружен — доступен живой LLM-режим").color(GOOD));
                if app.opponent.llm_model.is_empty() {
                    app.opponent.llm_model = "llama3.1".into();
                }
                ui.add(egui::TextEdit::singleline(&mut app.opponent.llm_model).desired_width(120.0));
            } else {
                ui.label(RichText::new("⚪ Ollama не запущен — работает офлайн-банк из 14 каверзных вопросов. Для живого режима: ollama.com → ollama pull llama3.1 → ollama serve").weak());
            }
        });
        ui.add_space(8.0);
        ui.separator();

        let n = opponent::QUESTIONS.len();
        let qs = &opponent::QUESTIONS[app.opponent.q_index % n];

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("[{}] ", qs.topic)).color(WARN).strong());
            ui.label(RichText::new(format!("вопрос {} из {}", app.opponent.q_index % n + 1, n)).weak());
        });
        ui.add_space(4.0);
        ui.label(RichText::new(qs.question).size(16.0));
        ui.add_space(8.0);

        let changed = egui::TextEdit::multiline(&mut app.opponent.answer)
            .desired_rows(5)
            .hint_text("Опиши механизм своими словами. Оппонент будет искать слабые места...")
            .show(ui)
            .response
            .changed();
        let _ = changed;

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(RichText::new("⚔ Отправить оппоненту").color(ACCENT))).clicked() {
                let v = opponent::evaluate(qs, &app.opponent.answer);
                let best = app.opponent.best_scores.entry(qs.topic.to_string()).or_insert(0);
                if v.score > *best { *best = v.score; }
                app.opponent.verdict = Some((v.score, v.critique));
                if llm_online && !app.opponent.answer.trim().is_empty() {
                    app.opponent.llm_in_flight = true;
                }
            }
            if ui.button("🎲 Другой вопрос").clicked() {
                app.opponent.q_index = (app.opponent.q_index + 1) % n;
                app.opponent.answer.clear();
                app.opponent.verdict = None;
                app.opponent.llm_reply = None;
            }
        });

        if let Some((score, critique)) = &app.opponent.verdict {
            ui.add_space(10.0);
            let color = if *score >= 70 { GOOD } else if *score >= 40 { WARN } else { ACCENT };
            egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(38, 30, 40)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Оценка: {score}/100")).color(color).strong());
                    ui.add(egui::ProgressBar::new(*score as f32 / 100.0).desired_width(200.0));
                });
                ui.label(critique);
            });
        }

        // LLM follow-up
        if app.opponent.llm_in_flight {
            ui.add_space(6.0);
            ui.label(RichText::new("🤖 Живой оппонент думает...").weak());
            let topic = qs.topic.to_string();
            let answer = app.opponent.answer.clone();
            let model = app.opponent.llm_model.clone();
            // блокирующий вызов в фоне отложим: для простоты выполняем синхронно по кнопке
            app.opponent.llm_in_flight = false;
            match opponent::ollama_ask(&model, &topic, &answer) {
                Ok(reply) => {
                    egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(30, 34, 46)).show(ui, |ui| {
                        ui.label(RichText::new("🤖 Ollama:").strong());
                        ui.label(&reply);
                    });
                    app.opponent.llm_reply = Some(reply);
                }
                Err(e) => {
                    app.opponent.llm_reply = Some(format!("(LLM недоступен: {e})"));
                }
            }
        } else if let Some(r) = &app.opponent.llm_reply {
            egui::Frame::group(ui.style()).fill(egui::Color32::from_rgb(30, 34, 46)).show(ui, |ui| {
                ui.label(RichText::new("🤖 Ollama:").strong());
                ui.label(r);
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label(RichText::new("📊 Лучшие баллы по темам:").strong());
        let mut rows: Vec<_> = app.opponent.best_scores.iter().collect();
        rows.sort_by_key(|(_, s)| std::cmp::Reverse(**s));
        for (topic, score) in rows {
            ui.horizontal(|ui| {
                ui.label(topic.as_str());
                ui.add(egui::ProgressBar::new(*score as f32 / 100.0).desired_width(160.0).text(format!("{score}")));
            });
        }
        if app.opponent.best_scores.is_empty() {
            ui.label(RichText::new("Пока пусто — ответь хотя бы на один вопрос.").weak());
        }
    });
}

fn sims_generative(app: &mut AppState, ui: &mut egui::Ui) {
    use crate::simulators::{generate, check_gen, GenKind};
    ui.add_space(8.0);
    ui.label(RichText::new("Бесконечные задачи с рандомизацией: каждый «Новый вопрос» — новая задача. Запомнить ответы невозможно — работает только навык.").weak());

    let kinds = [GenKind::RipRelative, GenKind::LittleEndian, GenKind::DecodeMov];
    ui.horizontal(|ui| {
        for (i, k) in kinds.iter().enumerate() {
            if ui.selectable_label(app.sim.gen_kind == i, k.title()).clicked() {
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
        ui.add(egui::TextEdit::singleline(&mut app.sim.gen_answer).desired_width(160.0).hint_text("hex, напр. 1a2b"));
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
        ui.label(RichText::new(if *ok { "✔ Верно! +10 XP" } else { "✘ Неверно" })
            .color(if *ok { GOOD } else { WARN }).strong());
        ui.label(RichText::new(explain).size(12.0));
    }
}

