// Анти-паттерн детектор: поведенческая аналитика обучения по progress.json.
// Говорит студенту жёстко и конкретно то, что никто не скажет в лицо.

use crate::curriculum::Curriculum;
use crate::state::Progress;

pub struct Finding {
    pub severity: Severity,
    pub title: String,
    pub advice: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warn,
    Alert,
}

impl Severity {
    pub fn icon(&self) -> &'static str {
        match self {
            Severity::Info => "💡",
            Severity::Warn => "⚠",
            Severity::Alert => "🚨",
        }
    }
    pub fn color32(&self) -> egui::Color32 {
        match self {
            Severity::Info => egui::Color32::from_rgb(120, 170, 230),
            Severity::Warn => egui::Color32::from_rgb(240, 180, 70),
            Severity::Alert => egui::Color32::from_rgb(220, 60, 70),
        }
    }
}

pub fn analyze(p: &Progress, _c: &Curriculum) -> Vec<Finding> {
    let mut out = Vec::new();

    let quizzes_done = p.quiz_correct.len();
    let psets = p.psets_done.len();
    let labs = p.lab_steps_done.len();
    let challenges_solved = p.challenges_solved.len();
    // дриллов в Progress нет (DrillState хранится в памяти сессии) — используем pset_explains как прокси честности
    let weeks = p.weeks_done.len();

    // Слишком мало данных для выводов
    if weeks == 0 && quizzes_done < 5 {
        return out;
    }

    // 1. ТЕОРЕТИК: много квизов — ноль запущенных бинарей
    if quizzes_done >= 10 && challenges_solved == 0 && labs == 0 {
        out.push(Finding {
            severity: Severity::Alert,
            title: "Паттерн «Теоретик»".into(),
            advice: format!(
                "Квизов отвечено: {quizzes_done}, но ни один бинарь не запущен и ни одна лаба не открыта. Знание реверса живёт в пальцах, а не в голове. Прямо сейчас: экспортируй первый челлендж (кнопка 📤) и открой его в Ghidra — даже если кажется рано."
            ),
        });
    }

    // 2. ИМИТАТОР: квизы на 100%, но лабы/дриллы пустые
    let explains = p.pset_explains.len();
    if quizzes_done >= 15 && explains == 0 && labs == 0 && challenges_solved == 0 {
        out.push(Finding {
            severity: Severity::Warn,
            title: "Дисбаланс «квизы >> практика»".into(),
            advice: format!(
                "Квизов: {quizzes_done}, self-explain: {explains}, лаб: {labs}. Квиз проверяет узнавание, практика — умение. Без тени практики на PSets тебя ждёт стена."
            ),
        });
    }

    // 3. ГАЛОМЕТКА: PSet закрыты, но journal пуст (нет self-explain следов)
    if psets >= 3 && p.journal.trim().len() < 50 {
        out.push(Finding {
            severity: Severity::Warn,
            title: "Подозрение на «галометку»".into(),
            advice: format!(
                "Problem sets закрыто: {psets}, но журнал почти пуст. Если решение не оставило следов в journal (что пробовал, где застрял, что понял) — оно скорее всего не твоё. Правило честности: закрыл PSet — запиши в journal одно предложение «почему это работает»."
            ),
        });
    }

    // 4. ПАССИВНОЕ ПОТРЕБЛЕНИЕ: недели отмечены, чекпоинтов мало
    let total_checkpoints: usize = p.checkpoint.values().map(|v| v.iter().filter(|b| **b).count()).sum();
    if weeks >= 2 && total_checkpoints < weeks * 2 {
        out.push(Finding {
            severity: Severity::Info,
            title: "Недели закрываются без чекпоинтов".into(),
            advice: "Ты отмечаешь недели пройденными, но чекпоинты внутри не отмечаются. Чекпоинт — это честная самопроверка: без неё «неделя пройдена» значит «неделя пролистана».".into(),
        });
    }

    // 5. ХРАНИЛИЩЕ КАРТОЧЕК: карточки не используются при ошибках в квизах
    if quizzes_done >= 10 && p.card_levels.is_empty() {
        out.push(Finding {
            severity: Severity::Info,
            title: "Флеш-карточки не задействованы".into(),
            advice: "Ни одной карточки в ротации. Кривая забывания съест пройденное за 2–3 недели. Отвечай «Повторить» на неверных квизах — ошибки сами превращаются в карточки.".into(),
        });
    }

    // 6. ПОЗИТИВ: сбалансированный прогресс — зафиксировать
    if challenges_solved >= 3 && labs >= 5 && quizzes_done >= 10 {
        out.push(Finding {
            severity: Severity::Info,
            title: "Баланс в норме ✔".into(),
            advice: format!(
                "Квизы: {quizzes_done}, лабы: {labs}, челленджи: {challenges_solved}. Так и выглядит обучение реверсу. Держи ритм: 2/3 практики, 1/3 теории."
            ),
        });
    }

    // 7. СТАГНАЦИЯ: стрик нулевой при большом xp — активность прерывалась давно
    if p.xp > 200 && p.streak.1 == 0 {
        out.push(Finding {
            severity: Severity::Warn,
            title: "Разрыв в практике".into(),
            advice: "XP накоплен, но стрик нулевой — вы не занимались недавно. Навык реверса деградирует быстрее, чем знание: 2 недели паузы = минус месяц. Начните с 15 минут дриллов, чтобы втянуться обратно.".into(),
        });
    }

    // Сортировка по убыванию серьёзности
    out.sort_by_key(|f| std::cmp::Reverse(f.severity.clone()));
    out
}
