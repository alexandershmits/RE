// Анти-паттерн детектор: поведенческая аналитика обучения по progress.json.
// Говорит студенту жёстко и конкретно то, что никто не скажет в лицо.

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
}

/// Дней без занятий, после которых детектор считает практику прерванной.
pub const STAGNATION_DAYS: u64 = 3;

/// `days_away` — сколько дней прошло с последней активности к моменту запуска приложения.
pub fn analyze(p: &Progress, days_away: u64) -> Vec<Finding> {
    let mut out = Vec::new();

    let quizzes_done = p.quiz_correct.len();
    let psets = p.psets_done.len();
    let labs = p.lab_steps_done.len();
    let challenges_solved = p.challenges_solved.len();
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
    if psets >= 3 && p.journal.trim().chars().count() < 50 {
        out.push(Finding {
            severity: Severity::Warn,
            title: "Подозрение на «галометку»".into(),
            advice: format!(
                "Problem sets закрыто: {psets}, но журнал почти пуст. Если решение не оставило следов в journal (что пробовал, где застрял, что понял) — оно скорее всего не твоё. Правило честности: закрыл PSet — запиши в journal одно предложение «почему это работает»."
            ),
        });
    }

    // 4. ПАССИВНОЕ ПОТРЕБЛЕНИЕ: недели отмечены, чекпоинтов мало
    let total_checkpoints: usize = p
        .checkpoint
        .values()
        .map(|v| v.iter().filter(|b| **b).count())
        .sum();
    if weeks >= 2 && total_checkpoints < weeks * 2 {
        out.push(Finding {
            severity: Severity::Info,
            title: "Недели закрываются без чекпоинтов".into(),
            advice: "Ты отмечаешь недели пройденными, но чекпоинты внутри не отмечаются. Чекпоинт — это честная самопроверка: без неё «неделя пройдена» значит «неделя пролистана».".into(),
        });
    }

    // 5. ХРАНИЛИЩЕ КАРТОЧЕК: карточки не используются при ошибках в квизах
    if quizzes_done >= 10 && p.card_due.is_empty() {
        out.push(Finding {
            severity: Severity::Info,
            title: "Флеш-карточки не задействованы".into(),
            advice: "Ни одной карточки в ротации. Кривая забывания съест пройденное за 2–3 недели. Неверные ответы в квизах сами попадают в колоду «Карточки» — заходи туда каждый день, пока колода не опустеет.".into(),
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

    // 7. СТАГНАЦИЯ: XP накоплен, но приложение не открывали несколько дней
    if p.xp > 200 && days_away >= STAGNATION_DAYS {
        out.push(Finding {
            severity: Severity::Warn,
            title: "Разрыв в практике".into(),
            advice: format!(
                "Последнее занятие было {days_away} дн. назад. Навык реверса деградирует быстрее, чем знание: 2 недели паузы = минус месяц. Начните с 15 минут дриллов, чтобы втянуться обратно."
            ),
        });
    }

    // Сортировка по убыванию серьёзности
    out.sort_by_key(|f| std::cmp::Reverse(f.severity.clone()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn titles(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.title.as_str()).collect()
    }

    fn with_quizzes(n: usize) -> Progress {
        let mut p = Progress::default();
        for i in 0..n {
            p.quiz_correct.insert(format!("q{i}"));
        }
        p
    }

    #[test]
    fn brand_new_profile_gets_no_verdicts() {
        assert!(analyze(&Progress::default(), 0).is_empty());
    }

    #[test]
    fn theorist_is_flagged_first() {
        let f = analyze(&with_quizzes(15), 0);
        assert!(titles(&f)[0].contains("Теоретик"), "{:?}", titles(&f));
        assert!(f[0].severity == Severity::Alert);
    }

    #[test]
    fn balanced_progress_is_recognised() {
        let mut p = with_quizzes(12);
        p.lab_steps_done.extend((0..6).map(|i| format!("w:lab{i}")));
        p.challenges_solved
            .extend((0..4).map(|i| format!("lv{i}a")));
        assert!(titles(&analyze(&p, 0)).iter().any(|t| t.contains("Баланс")));
    }

    #[test]
    fn closed_psets_with_an_empty_journal_look_suspicious() {
        let mut p = with_quizzes(6);
        p.psets_done.extend((0..3).map(|i| format!("w{i}:0")));
        p.journal = "Я".repeat(49); // 49 кириллических символов — это 98 байт, но всё ещё мало
        assert!(titles(&analyze(&p, 0))
            .iter()
            .any(|t| t.contains("галометк")));
        p.journal = "Я".repeat(50);
        assert!(!titles(&analyze(&p, 0))
            .iter()
            .any(|t| t.contains("галометк")));
    }

    #[test]
    fn a_real_break_is_reported_with_the_number_of_days() {
        // регресс: условие `streak == 0` было недостижимо, разрыв не находился никогда
        let mut p = with_quizzes(6);
        p.xp = 500;
        assert!(!titles(&analyze(&p, STAGNATION_DAYS - 1))
            .iter()
            .any(|t| t.contains("Разрыв")));
        let f = analyze(&p, 9);
        let gap = f
            .iter()
            .find(|f| f.title.contains("Разрыв"))
            .expect("разрыв в практике");
        assert!(gap.advice.contains("9 дн."));
        p.xp = 100;
        assert!(
            !titles(&analyze(&p, 9)).iter().any(|t| t.contains("Разрыв")),
            "мало XP — не о чем говорить"
        );
    }

    #[test]
    fn findings_are_sorted_by_severity() {
        let mut p = with_quizzes(15);
        p.xp = 900;
        let f = analyze(&p, 10);
        assert!(f.windows(2).all(|w| w[0].severity >= w[1].severity));
    }
}
