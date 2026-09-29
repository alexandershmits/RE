//! Тренировки: квизы, карточки, ре-экзамен, челленджи, рабочие сессии, оппонент.

use super::progress::WorkRecord;
use super::sessions::{Card, CardSession, QuizSession, ReexamState, WorkSession};
use super::{xp, AppState};
use crate::curriculum::Quiz;
use crate::rng::Rng;
use crate::util;
use std::collections::BTreeMap;

/// Интервал повторения карточки по её уровню, в днях.
pub const CARD_INTERVAL_DAYS: [u64; 6] = [0, 1, 3, 7, 14, 30];
/// С этого уровня карточка-ошибка считается выученной и уходит из колоды.
pub const MISTAKE_GRADUATION_LEVEL: u8 = 3;
pub const REEXAM_QUESTIONS: usize = 10;
pub const REEXAM_PASS_PERCENT: u32 = 70;
pub const REEXAM_PERIOD_DAYS: u64 = 30;
/// Минимальная полнота отчёта, при которой тикет можно закрыть.
pub const WORK_MIN_COMPLETENESS: u8 = 30;
const WORK_HISTORY_LIMIT: usize = 200;

/// Флаг в любом виде: `FLAG{x}`, `flag{x}` или просто `x`.
pub fn normalize_flag(input: &str) -> &str {
    let s = input.trim();
    match s.get(..5) {
        Some(prefix) if prefix.eq_ignore_ascii_case("FLAG{") && s.ends_with('}') => {
            &s[5..s.len() - 1]
        }
        _ => s,
    }
}

impl AppState {
    // ── квизы ──

    /// Запускает сессию; `false`, если в выбранном модуле нет вопросов.
    pub fn start_quiz(&mut self, filter_module: Option<u8>) -> bool {
        self.quiz = QuizSession::new(
            &self.curriculum.quizzes,
            filter_module,
            &mut Rng::from_time(),
        );
        self.quiz.is_some()
    }

    pub fn submit_quiz_answer(&mut self, quiz: &Quiz, selected: usize) -> bool {
        let correct = selected == quiz.correct;
        self.progress.quiz_answers.insert(quiz.id.clone(), selected);
        if correct {
            self.progress.quiz_correct.insert(quiz.id.clone());
        } else {
            self.note_mistake(&quiz.id);
        }
        self.mark_dirty();
        self.check_achievements();
        correct
    }

    /// Ошибка попадает в колоду карточек и требует повторения сегодня.
    fn note_mistake(&mut self, quiz_id: &str) {
        self.progress.mistakes.insert(quiz_id.to_string());
        let key = format!("quiz:{quiz_id}");
        self.progress.card_levels.insert(key.clone(), 0);
        self.progress.card_due.remove(&key);
        self.mark_dirty();
    }

    /// Три модуля с наибольшим числом невыправленных ошибок (в квизе или на re-exam). Ошибка, исправленная
    /// верным ответом, не считается; новая ошибка в ранее решённом вопросе — считается.
    pub fn weak_topics(&self) -> Vec<(String, usize)> {
        let mut wrong: BTreeMap<u8, usize> = BTreeMap::new();
        for q in &self.curriculum.quizzes {
            let last_answer_wrong = self
                .progress
                .quiz_answers
                .get(&q.id)
                .is_some_and(|&a| a != q.correct);
            if self.progress.mistakes.contains(&q.id)
                && (!self.progress.quiz_correct.contains(&q.id) || last_answer_wrong)
            {
                *wrong.entry(q.module).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = wrong
            .into_iter()
            .map(|(m, n)| (self.curriculum.module_name(m), n))
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.1));
        v.truncate(3);
        v
    }

    // ── карточки: система Лейтнера ──

    /// Обычные карточки + карточки-ошибки из квизов.
    pub fn deck(&self) -> Vec<Card> {
        let mut cards: Vec<Card> = self
            .curriculum
            .flashcards
            .iter()
            .map(|f| Card {
                id: f.id.clone(),
                front: f.front.clone(),
                back: f.back.clone(),
                tags: f.tags.clone(),
                mistake: false,
            })
            .collect();
        for id in &self.progress.mistakes {
            let Some(q) = self.curriculum.quiz_by_id(id) else {
                continue;
            };
            let answer = q.answers.get(q.correct).map_or("?", String::as_str);
            cards.push(Card {
                id: format!("quiz:{id}"),
                front: q.question.clone(),
                back: format!("{answer}\n\n{}", q.explain),
                tags: vec![
                    "ошибка в квизе".into(),
                    self.curriculum.module_name(q.module),
                ],
                mistake: true,
            });
        }
        cards
    }

    pub fn card_level(&self, id: &str) -> u8 {
        self.progress.card_levels.get(id).copied().unwrap_or(0)
    }

    /// Ни разу не повторённая карточка «к повторению» с первого дня.
    pub fn card_due_day(&self, id: &str) -> u64 {
        self.progress.card_due.get(id).copied().unwrap_or(0)
    }

    fn ordered(&self, mut cards: Vec<Card>, rng: &mut Rng) -> Vec<Card> {
        rng.shuffle(&mut cards);
        cards.sort_by_key(|c| (!c.mistake, self.card_level(&c.id)));
        cards
    }

    /// Карточки, срок которых наступил: ошибки первыми, затем слабые.
    pub fn due_deck(&self, today: u64, rng: &mut Rng) -> Vec<Card> {
        let due = self
            .deck()
            .into_iter()
            .filter(|c| self.card_due_day(&c.id) <= today)
            .collect();
        self.ordered(due, rng)
    }

    /// Запускает сессию по карточкам. `false` — повторять нечего.
    pub fn start_cards(&mut self, only_due: bool, today: u64) -> bool {
        let mut rng = Rng::from_time();
        let cards = if only_due {
            self.due_deck(today, &mut rng)
        } else {
            let all = self.deck();
            self.ordered(all, &mut rng)
        };
        if cards.is_empty() {
            return false;
        }
        self.cards = Some(CardSession::new(cards));
        true
    }

    pub fn review_card(&mut self, id: &str, known: bool, today: u64) {
        // «Повторить всё равно» не двигает расписание: без этого три «Знаю» за день выпускали ошибку из колоды
        if known && self.card_due_day(id) > today {
            return;
        }
        let level = self.card_level(id);
        let new_level = if known {
            level.saturating_add(1).min(5)
        } else {
            level.saturating_sub(1)
        };
        let due = if known {
            today + CARD_INTERVAL_DAYS[usize::from(new_level)]
        } else {
            today
        };
        self.progress.card_levels.insert(id.to_string(), new_level);
        self.progress.card_due.insert(id.to_string(), due);
        if known && new_level >= MISTAKE_GRADUATION_LEVEL {
            if let Some(quiz_id) = id.strip_prefix("quiz:") {
                self.progress.mistakes.remove(quiz_id);
            }
        }
        self.mark_dirty();
    }

    // ── ре-сертификация ──

    /// Прошло ли 30 дней с последнего экзамена (или экзамена ещё не было, а XP уже накоплен).
    pub fn reexam_due(&self, day: u64) -> bool {
        match self.progress.last_reexam_day {
            Some(d) => day.saturating_sub(d) >= REEXAM_PERIOD_DAYS,
            None => self.progress.xp > 300,
        }
    }

    /// 10 вопросов: сначала уже решённые верно, при нехватке — остальные.
    pub fn start_reexam(&mut self, seed: u64) {
        let mut rng = Rng::new(seed);
        let (mut pool, mut rest): (Vec<Quiz>, Vec<Quiz>) = self
            .curriculum
            .quizzes
            .iter()
            .cloned()
            .partition(|q| self.progress.quiz_correct.contains(&q.id));
        rng.shuffle(&mut pool);
        rng.shuffle(&mut rest);
        pool.extend(rest);
        pool.truncate(REEXAM_QUESTIONS);
        self.reexam = Some(ReexamState {
            questions: pool,
            pos: 0,
            selected: None,
            correct: 0,
            answered: false,
            finished: false,
            wrong: Vec::new(),
        });
    }

    pub fn reexam_answer(&mut self, selected: usize) {
        let Some(rx) = self.reexam.as_mut() else {
            return;
        };
        let Some(q) = rx.questions.get(rx.pos) else {
            return;
        };
        if rx.answered {
            return;
        }
        rx.answered = true;
        rx.selected = Some(selected);
        if selected == q.correct {
            rx.correct += 1;
        } else {
            rx.wrong.push(q.id.clone());
        }
    }

    pub fn reexam_next(&mut self) {
        if let Some(rx) = self.reexam.as_mut() {
            rx.pos += 1;
            rx.selected = None;
            rx.answered = false;
        }
    }

    /// Подводит итог, возвращает текст уведомления. Ошибочные вопросы возвращаются в слабые темы.
    pub fn finish_reexam(&mut self, today: u64) -> String {
        let Some(rx) = self.reexam.as_mut() else {
            return String::new();
        };
        rx.finished = true;
        let (correct, total) = (rx.correct, rx.questions.len() as u32);
        let wrong = std::mem::take(&mut rx.wrong);
        for id in wrong {
            self.progress.quiz_correct.remove(&id);
            self.note_mistake(&id);
        }
        self.progress.reexam_score = Some((util::format_day(today), correct, total));
        self.progress.last_reexam_day = Some(today);
        self.mark_dirty();
        if correct * 100 >= total * REEXAM_PASS_PERCENT {
            format!("Re-certification: {correct}/{total} — порог {REEXAM_PASS_PERCENT}% пройден ✔")
        } else {
            format!(
                "Re-certification: {correct}/{total}. Порог {REEXAM_PASS_PERCENT}% не пройден — \
                 ошибочные вопросы возвращены в слабые темы и карточки."
            )
        }
    }

    // ── челленджи ──

    pub fn submit_flag(&mut self, id: &str, flag: &str) -> bool {
        let Some(ch) = self.curriculum.challenges.iter().find(|c| c.id == id) else {
            return false;
        };
        if ch.flag.is_empty() || normalize_flag(flag) != ch.flag {
            return false;
        }
        if self.progress.challenges_solved.insert(id.to_string()) {
            self.add_xp(xp::FLAG);
        }
        self.mark_dirty();
        self.check_achievements();
        true
    }

    // ── XP за тренировки ──

    /// Дрилл засчитывается (и платит) один раз.
    pub fn solve_drill(&mut self, key: &str, reward: u32) -> bool {
        let first = self.progress.drills_solved.insert(key.to_string());
        if first {
            self.add_xp(reward);
        }
        first
    }

    /// Симулятор платит один раз за задачу (`reg0`, `pe2`, `oep1`, …).
    pub fn award_sim(&mut self, key: &str, reward: u32) -> bool {
        self.award_once(&format!("sim:{key}"), reward)
    }

    /// Бесконечный генератор платит за `GENERATED_PER_DAY` задач в день.
    pub fn award_generated(&mut self, today: u64) -> bool {
        if self.generated_left(today) == 0 {
            return false;
        }
        let (day, count) = self.progress.gen_practice;
        self.progress.gen_practice = (today, if day == today { count + 1 } else { 1 });
        self.add_xp(xp::GENERATED);
        true
    }

    pub fn generated_left(&self, today: u64) -> u32 {
        let (day, count) = self.progress.gen_practice;
        let used = if day == today { count } else { 0 };
        xp::GENERATED_PER_DAY.saturating_sub(used)
    }

    pub fn record_opponent_score(&mut self, topic: &str, score: u8) {
        let best = self
            .progress
            .opponent_best
            .entry(topic.to_string())
            .or_insert(0);
        if score > *best {
            *best = score;
            self.mark_dirty();
        }
    }

    // ── рабочая сессия ──

    /// Тикет — случайный нерешённый челлендж (если всё решено — любой).
    pub fn start_work_session(&mut self, now: u64, seed: u64) {
        let unsolved: Vec<&str> = self
            .curriculum
            .challenges
            .iter()
            .filter(|c| !self.progress.challenges_solved.contains(&c.id))
            .map(|c| c.id.as_str())
            .collect();
        let pool: Vec<&str> = if unsolved.is_empty() {
            self.curriculum
                .challenges
                .iter()
                .map(|c| c.id.as_str())
                .collect()
        } else {
            unsolved
        };
        if pool.is_empty() {
            return;
        }
        let id = pool[Rng::new(seed).below(pool.len() as u64) as usize].to_string();
        self.work = WorkSession {
            active: true,
            challenge_id: id,
            started_unix: now,
            ..WorkSession::new()
        };
    }

    /// Тикет можно закрыть: пройдены триаж и статика, есть гипотеза и достаточно полный отчёт.
    pub fn work_can_finish(&self) -> bool {
        let done = |stage: &str| self.work.stages_done.iter().any(|s| s == stage);
        self.work.report_completeness() >= WORK_MIN_COMPLETENESS
            && !self.work.hypotheses.is_empty()
            && done("triage")
            && done("static")
    }

    /// Закрывает тикет и возвращает итоговое сообщение. XP за тикет платится один раз.
    pub fn finish_work_session(&mut self, now: u64) -> String {
        if !self.work.active {
            return String::new();
        }
        if !self.work_can_finish() {
            return "Тикет нельзя закрыть: отметьте этапы «триаж» и «статика», запишите гипотезу и заполните 3 пункта отчёта.".into();
        }
        let seconds = now.saturating_sub(self.work.started_unix);
        let completeness = self.work.report_completeness();
        let method_ok = self.work.methodology_ok().is_ok();
        let id = std::mem::take(&mut self.work.challenge_id);
        self.progress.work_history.push(WorkRecord {
            challenge: id.clone(),
            seconds,
            method_ok,
            completeness,
        });
        if self.progress.work_history.len() > WORK_HISTORY_LIMIT {
            self.progress.work_history.remove(0);
        }
        let reward = xp::WORK_BASE
            + if completeness >= 70 {
                xp::WORK_BONUS
            } else {
                0
            };
        let paid = self.award_once(&format!("work:{id}"), reward);
        self.work.active = false;
        self.mark_dirty();
        let mut msg =
            format!("Тикет {id} закрыт за {seconds} сек. Полнота отчёта: {completeness}%.");
        msg.push_str(if method_ok {
            " Методология в порядке ✔"
        } else {
            " ⚠ Нарушение методологии — см. замечание."
        });
        if paid {
            msg.push_str(&format!(" +{reward} XP"));
        } else {
            msg.push_str(" XP за этот тикет уже получен.");
        }
        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiz(app: &AppState, i: usize) -> Quiz {
        app.curriculum.quizzes[i].clone()
    }

    fn wrong_answer(q: &Quiz) -> usize {
        (q.correct + 1) % q.answers.len()
    }

    fn fill_report(app: &mut AppState, points: usize) {
        for i in 0..points {
            app.work.report[i] = format!("пункт отчёта номер {i}");
        }
    }

    #[test]
    fn wrong_answer_becomes_a_mistake_card_due_today() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 0);
        assert!(!app.submit_quiz_answer(&q, wrong_answer(&q)));
        assert!(app.progress.mistakes.contains(&q.id));
        let card = app
            .deck()
            .into_iter()
            .find(|c| c.id == format!("quiz:{}", q.id))
            .expect("карточка-ошибка");
        assert!(
            card.mistake
                && card.back.contains(&q.answers[q.correct])
                && card.back.contains(&q.explain)
        );
        assert!(app
            .due_deck(10, &mut Rng::new(1))
            .iter()
            .any(|c| c.id == card.id));
    }

    #[test]
    fn correct_answer_creates_no_card() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 0);
        assert!(app.submit_quiz_answer(&q, q.correct));
        assert!(app.progress.mistakes.is_empty() && app.progress.quiz_correct.contains(&q.id));
    }

    #[test]
    fn card_intervals_grow_and_mistake_graduates() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 0);
        app.submit_quiz_answer(&q, wrong_answer(&q));
        let id = format!("quiz:{}", q.id);
        let mut today = 100;
        for (expected_level, days) in [(1u8, 1u64), (2, 3), (3, 7)] {
            app.review_card(&id, true, today);
            assert_eq!(app.card_level(&id), expected_level);
            assert_eq!(app.card_due_day(&id), today + days);
            today += days;
        }
        assert!(
            !app.progress.mistakes.contains(&q.id),
            "после третьего успеха ошибка «выучена»"
        );
        assert!(app.deck().iter().all(|c| c.id != id));
    }

    #[test]
    fn extra_practice_does_not_advance_the_schedule() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 0);
        app.submit_quiz_answer(&q, wrong_answer(&q));
        let id = format!("quiz:{}", q.id);
        for _ in 0..5 {
            app.review_card(&id, true, 100); // «Повторить всё равно»: срок наступил только в первый раз
        }
        assert_eq!((app.card_level(&id), app.card_due_day(&id)), (1, 101));
        assert!(
            app.progress.mistakes.contains(&q.id),
            "пять «Знаю» за один день не выпускают ошибку"
        );
        app.review_card(&id, true, 101);
        assert_eq!(
            app.card_level(&id),
            2,
            "когда срок пришёл, расписание идёт дальше"
        );
    }

    #[test]
    fn a_new_mistake_in_a_solved_quiz_is_a_weak_topic() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 0);
        let weak = |app: &AppState| app.weak_topics().iter().map(|(_, n)| n).sum::<usize>();
        app.submit_quiz_answer(&q, q.correct);
        assert_eq!(weak(&app), 0);
        app.submit_quiz_answer(&q, wrong_answer(&q));
        assert_eq!(
            weak(&app),
            1,
            "решённый ранее вопрос, в котором ошиблись снова"
        );
        app.submit_quiz_answer(&q, q.correct);
        assert_eq!(weak(&app), 0, "верный ответ исправляет ошибку");
    }

    #[test]
    fn trainer_hint_matches_the_card_schedule() {
        // текст подсказки в тренажёре: «сегодня, затем через 1 и 3 дня; после третьего верного повтора уйдёт»
        assert_eq!(CARD_INTERVAL_DAYS[1..=2], [1, 3]);
        assert_eq!(MISTAKE_GRADUATION_LEVEL, 3);
    }

    #[test]
    fn forgotten_card_drops_a_level_and_stays_due() {
        let mut app = AppState::in_memory();
        let id = app.curriculum.flashcards[0].id.clone();
        app.review_card(&id, true, 50);
        app.review_card(&id, true, 51);
        assert_eq!(app.card_level(&id), 2);
        app.review_card(&id, false, 54);
        assert_eq!((app.card_level(&id), app.card_due_day(&id)), (1, 54));
        app.review_card(&id, false, 54);
        app.review_card(&id, false, 54);
        assert_eq!(app.card_level(&id), 0, "ниже нуля не падает");
    }

    #[test]
    fn unseen_flashcards_are_due_and_mistakes_come_first() {
        let mut app = AppState::in_memory();
        let q = quiz(&app, 3);
        app.submit_quiz_answer(&q, wrong_answer(&q));
        let due = app.due_deck(1, &mut Rng::new(9));
        assert_eq!(due.len(), app.curriculum.flashcards.len() + 1);
        assert!(due[0].mistake && due[1..].iter().all(|c| !c.mistake));
        let first = app.curriculum.flashcards[0].id.clone();
        app.review_card(&first, true, 1);
        assert!(
            app.due_deck(1, &mut Rng::new(9))
                .iter()
                .all(|c| c.id != first),
            "повторённая карточка уходит до завтра"
        );
    }

    #[test]
    fn card_session_is_empty_when_nothing_is_due() {
        let mut app = AppState::in_memory();
        for c in app.deck() {
            app.review_card(&c.id, true, 5);
        }
        assert!(!app.start_cards(true, 5));
        assert!(
            app.start_cards(false, 5),
            "«повторить всё равно» работает всегда"
        );
        assert_eq!(
            app.cards.as_ref().map(|s| s.cards.len()),
            Some(app.curriculum.flashcards.len())
        );
    }

    #[test]
    fn failed_reexam_returns_wrong_questions_to_weak_topics() {
        let mut app = AppState::in_memory();
        for q in app.curriculum.quizzes.clone() {
            app.progress.quiz_correct.insert(q.id);
        }
        app.start_reexam(7);
        let n = app.reexam.as_ref().unwrap().questions.len();
        assert_eq!(n, REEXAM_QUESTIONS);
        for i in 0..n {
            let q = app.reexam.as_ref().unwrap().questions[i].clone();
            app.reexam_answer(if i < 2 { q.correct } else { wrong_answer(&q) });
            app.reexam_next();
        }
        let msg = app.finish_reexam(20_000);
        assert!(msg.contains("не пройден"), "{msg}");
        assert_eq!(
            app.progress.reexam_score,
            Some(("2024-10-04".into(), 2, 10))
        );
        assert_eq!(app.progress.mistakes.len(), 8);
        assert_eq!(
            app.progress.quiz_correct.len(),
            app.curriculum.quizzes.len() - 8
        );
        assert_eq!(app.progress.last_reexam_day, Some(20_000));
        let weak: usize = app.weak_topics().iter().map(|(_, n)| n).sum();
        assert!(
            (3..=8).contains(&weak),
            "ошибки re-exam должны быть в слабых темах (топ-3 модуля): {weak}"
        );
    }

    #[test]
    fn passed_reexam_keeps_progress_and_schedules_next_in_30_days() {
        let mut app = AppState::in_memory();
        assert!(!app.reexam_due(100), "новичку экзамен не нужен");
        app.progress.xp = 500;
        assert!(app.reexam_due(100));
        app.start_reexam(1);
        for _ in 0..REEXAM_QUESTIONS {
            let q =
                app.reexam.as_ref().unwrap().questions[app.reexam.as_ref().unwrap().pos].clone();
            app.reexam_answer(q.correct);
            app.reexam_answer(wrong_answer(&q)); // повторный ответ на тот же вопрос игнорируется
            app.reexam_next();
        }
        assert!(app.finish_reexam(100).contains("пройден ✔"));
        assert!(app.progress.mistakes.is_empty());
        assert!(!app.reexam_due(129) && app.reexam_due(130));
    }

    #[test]
    fn reexam_prefers_questions_the_student_already_solved() {
        let mut app = AppState::in_memory();
        let solved: Vec<String> = app
            .curriculum
            .quizzes
            .iter()
            .take(12)
            .map(|q| q.id.clone())
            .collect();
        app.progress.quiz_correct.extend(solved.iter().cloned());
        app.start_reexam(5);
        assert!(app
            .reexam
            .unwrap()
            .questions
            .iter()
            .all(|q| solved.contains(&q.id)));
    }

    #[test]
    fn ticket_needs_hypothesis_and_report_then_pays_once() {
        let mut app = AppState::in_memory();
        app.start_work_session(1_000, 1);
        assert!(app.work.active && !app.work.challenge_id.is_empty());
        assert!(!app.work_can_finish());
        assert!(app.finish_work_session(1_100).contains("нельзя"));
        assert!(app.work.active, "неудачное закрытие не сбрасывает сессию");
        fill_report(&mut app, 3);
        assert!(!app.work_can_finish(), "без гипотезы тикет не закрыть");
        app.work.hypotheses.push("проверка через strcmp".into());
        assert!(
            !app.work_can_finish(),
            "без триажа и статики тикет не закрыть"
        );
        app.work.stages_done = vec!["triage".into(), "static".into(), "dynamic".into()];
        assert!(app.work_can_finish());
        let msg = app.finish_work_session(1_130);
        assert!(
            msg.contains("за 130 сек") && msg.contains("+30 XP"),
            "{msg}"
        );
        assert_eq!(app.progress.xp, xp::WORK_BASE);
        assert_eq!(app.progress.work_history.len(), 1);
        assert!(app.progress.work_history[0].method_ok);
        let id = app.progress.work_history[0].challenge.clone();

        app.work = WorkSession {
            active: true,
            challenge_id: id,
            started_unix: 2_000,
            stages_done: vec!["triage".into(), "static".into()],
            ..WorkSession::new()
        };
        fill_report(&mut app, 8);
        app.work.hypotheses.push("ещё раз".into());
        assert!(app.finish_work_session(2_010).contains("уже получен"));
        assert_eq!(
            app.progress.xp,
            xp::WORK_BASE,
            "повторный тикет по тому же файлу XP не даёт"
        );
        assert_eq!(app.progress.work_history.len(), 2);
    }

    #[test]
    fn strong_report_earns_the_bonus() {
        let mut app = AppState::in_memory();
        app.start_work_session(0, 2);
        fill_report(&mut app, 8);
        app.work.hypotheses.push("x".into());
        app.work.stages_done = vec!["triage".into(), "static".into()];
        assert!(app.finish_work_session(500).contains("+50 XP"));
    }

    #[test]
    fn ticket_prefers_unsolved_challenges() {
        let mut app = AppState::in_memory();
        let ids: Vec<String> = app
            .curriculum
            .challenges
            .iter()
            .map(|c| c.id.clone())
            .collect();
        app.progress
            .challenges_solved
            .extend(ids[1..].iter().cloned());
        for seed in 0..20 {
            app.start_work_session(0, seed);
            assert_eq!(app.work.challenge_id, ids[0]);
        }
    }

    #[test]
    fn flags_are_normalized_and_pay_once() {
        let mut app = AppState::in_memory();
        let ch = app.curriculum.challenges[0].clone();
        assert!(!app.submit_flag(&ch.id, ""));
        assert!(!app.submit_flag(&ch.id, "FLAG{wrong}"));
        assert!(!app.submit_flag("нет-такого", &ch.flag));
        assert!(app.submit_flag(&ch.id, &format!("  flag{{{}}} ", ch.flag)));
        assert!(app.submit_flag(&ch.id, &ch.flag));
        assert_eq!(app.progress.xp, xp::FLAG);
        assert_eq!(normalize_flag("FLAG{ab}"), "ab");
        assert_eq!(normalize_flag("ab"), "ab");
        assert_eq!(
            normalize_flag("флаг{ab}"),
            "флаг{ab}",
            "многобайтные символы не должны паниковать"
        );
    }

    #[test]
    fn drills_and_sims_pay_once_per_task() {
        let mut app = AppState::in_memory();
        assert!(app.solve_drill("asm3", xp::DRILL));
        assert!(!app.solve_drill("asm3", xp::DRILL));
        assert!(app.award_sim("reg0", xp::SIM_REGS));
        assert!(!app.award_sim("reg0", xp::SIM_REGS));
        assert_eq!(app.progress.xp, xp::DRILL + xp::SIM_REGS);
        assert_eq!(app.drills_solved_in("asm"), 1);
    }

    #[test]
    fn endless_generator_has_a_daily_xp_cap() {
        let mut app = AppState::in_memory();
        for _ in 0..xp::GENERATED_PER_DAY {
            assert!(app.award_generated(7));
        }
        assert!(!app.award_generated(7));
        assert_eq!(app.generated_left(7), 0);
        assert_eq!(app.progress.xp, xp::GENERATED * xp::GENERATED_PER_DAY);
        assert!(app.award_generated(8), "новый день — новый лимит");
        assert_eq!(app.generated_left(8), xp::GENERATED_PER_DAY - 1);
    }

    #[test]
    fn opponent_remembers_best_score_per_topic() {
        let mut app = AppState::in_memory();
        app.record_opponent_score("PE", 40);
        app.record_opponent_score("PE", 70);
        app.record_opponent_score("PE", 50);
        assert_eq!(app.progress.opponent_best["PE"], 70);
    }

    #[test]
    fn weak_topics_rank_modules_by_unfixed_mistakes() {
        let mut app = AppState::in_memory();
        let by_module = |m: u8| -> Vec<Quiz> {
            app.curriculum
                .quizzes
                .iter()
                .filter(|q| q.module == m)
                .cloned()
                .collect()
        };
        let (m1, m2) = (by_module(1), by_module(2));
        for q in m1.iter().take(3).chain(m2.iter().take(1)) {
            app.submit_quiz_answer(q, wrong_answer(q));
        }
        app.submit_quiz_answer(&m1[0], m1[0].correct); // исправлено — не считается
        let weak = app.weak_topics();
        assert_eq!(weak.len(), 2);
        assert_eq!(weak[0].1, 2);
        assert_eq!(weak[1].1, 1);
    }
}
