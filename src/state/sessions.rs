//! Состояние интерактивных сессий (не сохраняется между запусками).

use crate::curriculum::Quiz;
use crate::jobs::Job;
use crate::rng::Rng;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Tab {
    #[default]
    Dashboard,
    Course,
    Trainer,
    Achievements,
    Resources,
    Journal,
    Cards,
    Interview,
    Rubric,
    Diagrams,
    Placement,
    Sims,
    Drills,
    Challenges,
    Reexam,
    Opponent,
    Work,
}

impl Tab {
    pub const ALL: [Tab; 17] = [
        Tab::Dashboard,
        Tab::Course,
        Tab::Trainer,
        Tab::Achievements,
        Tab::Resources,
        Tab::Journal,
        Tab::Cards,
        Tab::Interview,
        Tab::Rubric,
        Tab::Diagrams,
        Tab::Placement,
        Tab::Sims,
        Tab::Drills,
        Tab::Challenges,
        Tab::Reexam,
        Tab::Opponent,
        Tab::Work,
    ];
}

/// Сессия тренажёра-квиза.
pub struct QuizSession {
    /// Индексы вопросов в `Curriculum::quizzes` в порядке показа.
    pub order: Vec<usize>,
    pub pos: usize,
    pub selected: Option<usize>,
    pub submitted: bool,
    pub correct_count: usize,
    pub answered: usize,
    pub filter_module: Option<u8>,
    pub finished: bool,
}

impl QuizSession {
    /// Перемешанные вопросы выбранного модуля (или всех). `None` — вопросов нет.
    pub fn new(quizzes: &[Quiz], filter_module: Option<u8>, rng: &mut Rng) -> Option<Self> {
        let mut order: Vec<usize> = quizzes
            .iter()
            .enumerate()
            .filter(|(_, q)| filter_module.is_none_or(|m| q.module == m))
            .map(|(i, _)| i)
            .collect();
        if order.is_empty() {
            return None;
        }
        rng.shuffle(&mut order);
        Some(QuizSession {
            order,
            pos: 0,
            selected: None,
            submitted: false,
            correct_count: 0,
            answered: 0,
            filter_module,
            finished: false,
        })
    }

    /// Индекс текущего вопроса в `Curriculum::quizzes`.
    pub fn current(&self) -> Option<usize> {
        if self.finished {
            None
        } else {
            self.order.get(self.pos).copied()
        }
    }

    pub fn record(&mut self, selected: usize, correct: bool) {
        self.selected = Some(selected);
        self.submitted = true;
        self.answered += 1;
        if correct {
            self.correct_count += 1;
        }
    }

    pub fn advance(&mut self) {
        self.pos += 1;
        self.selected = None;
        self.submitted = false;
        if self.pos >= self.order.len() {
            self.finished = true;
        }
    }

    /// Досрочное завершение: уже данные ответы сохраняются.
    pub fn abort(&mut self) {
        self.finished = true;
    }
}

/// Карточка колоды: обычная или «ошибка» из квиза.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub id: String,
    pub front: String,
    pub back: String,
    pub tags: Vec<String>,
    pub mistake: bool,
}

pub struct CardSession {
    pub cards: Vec<Card>,
    pub pos: usize,
    pub show_back: bool,
    pub known: Vec<String>,
    pub again: Vec<Card>,
}

impl CardSession {
    pub fn new(cards: Vec<Card>) -> Self {
        CardSession {
            cards,
            pos: 0,
            show_back: false,
            known: Vec::new(),
            again: Vec::new(),
        }
    }

    pub fn current(&self) -> Option<&Card> {
        self.cards.get(self.pos)
    }

    pub fn is_finished(&self) -> bool {
        self.pos >= self.cards.len()
    }
}

/// Ежемесячная рекертификация: 10 случайных задач из пройденного материала.
pub struct ReexamState {
    pub questions: Vec<Quiz>,
    pub pos: usize,
    pub selected: Option<usize>,
    pub correct: u32,
    pub answered: bool,
    pub finished: bool,
    /// Квизы, на которых студент ошибся.
    pub wrong: Vec<String>,
}

/// Симулятор рабочей сессии аналитика: тикет -> методология -> отчёт.
#[derive(Default)]
pub struct WorkSession {
    pub active: bool,
    /// Какой бинарь «пришёл» по тикету.
    pub challenge_id: String,
    pub started_unix: u64,
    /// Закрытые этапы в порядке выполнения: `triage`, `static`, `dynamic`, `report`.
    pub stages_done: Vec<String>,
    pub hypotheses: Vec<String>,
    pub triage_notes: String,
    pub static_notes: String,
    pub dynamic_notes: String,
    /// По rubric (10 пунктов).
    pub report: [String; 10],
    pub flag_found: bool,
}

impl WorkSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Порядок этапов корректен? (триаж и статика раньше динамики)
    pub fn methodology_ok(&self) -> Result<(), String> {
        let pos = |s: &str| self.stages_done.iter().position(|x| x == s);
        if let (Some(t), Some(d)) = (pos("triage"), pos("dynamic")) {
            if d < t {
                return Err("Динамика запущена ДО триажа. В реальной работе это риск: неизвестный сэмпл без первичной оценки = compro.".into());
            }
        }
        if let (Some(s), Some(d)) = (pos("static"), pos("dynamic")) {
            if d < s {
                return Err("Динамика до статики. Сначала гипотезы из статики, потом проверка — иначе ты не аналитик, а запускальщик.".into());
            }
        }
        Ok(())
    }

    /// Доля заполненных пунктов rubric в процентах (пункт засчитан от 10 символов).
    pub fn report_completeness(&self) -> u8 {
        let filled = self
            .report
            .iter()
            .filter(|s| s.trim().chars().count() >= 10)
            .count();
        (filled * 10) as u8
    }
}

/// Socratic-оппонент: вопрос, ответ студента, оценка, живая модель.
#[derive(Default)]
pub struct OpponentState {
    pub q_index: usize,
    pub answer: String,
    /// (балл, критика)
    pub verdict: Option<(u8, String)>,
    pub llm_reply: Option<String>,
    pub llm_model: String,
    pub llm_job: Option<Job<Result<String, String>>>,
    pub ollama_online: Option<bool>,
    pub ollama_job: Option<Job<bool>>,
    pub ollama_checked: Option<Instant>,
}

pub struct PlacementState {
    pub pos: usize,
    pub score: usize,
    pub selected: Option<usize>,
    pub answered: bool,
}

#[derive(Default)]
pub struct DrillState {
    /// 0=asm 1=addr 2=pattern 3=script
    pub which: usize,
    pub idx: usize,
    pub choice: Option<usize>,
    pub checked: bool,
    pub show_answer: bool,
}

#[derive(Default)]
pub struct SimState {
    /// 0=регистры 1=PE 2=OEP 3=генератор
    pub which: usize,
    /// 0=rip 1=le 2=decode
    pub gen_kind: usize,
    pub gen_seed: u64,
    pub gen_answer: String,
    /// (верно?, пояснение)
    pub gen_feedback: Option<(bool, String)>,
    pub task_idx: usize,
    pub show_answer: bool,
    /// Регистр -> введённое hex-значение.
    pub user_input: BTreeMap<String, String>,
    pub choice: Option<usize>,
    pub checked: bool,
    pub last_ok: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitKind {
    Week,
    Quiz,
    Challenge,
    Resource,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub title: String,
    pub kind: HitKind,
    pub week_id: String,
}

/// Итог фоновой генерации челленджей.
pub struct GeneratorReport {
    pub ok: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curriculum::Curriculum;

    fn quizzes() -> Vec<Quiz> {
        Curriculum::load().quizzes
    }

    #[test]
    fn module_filter_selects_only_that_module() {
        let all = quizzes();
        let s = QuizSession::new(&all, Some(3), &mut Rng::new(1)).unwrap();
        assert!(!s.order.is_empty());
        assert!(s.order.len() < all.len(), "фильтр должен сужать выборку");
        assert!(s.order.iter().all(|&i| all[i].module == 3));
        assert_eq!(s.order.len(), all.iter().filter(|q| q.module == 3).count());
    }

    #[test]
    fn no_filter_takes_every_quiz_once() {
        let all = quizzes();
        let mut order = QuizSession::new(&all, None, &mut Rng::new(2))
            .unwrap()
            .order;
        order.sort_unstable();
        assert_eq!(order, (0..all.len()).collect::<Vec<_>>());
    }

    #[test]
    fn module_without_quizzes_yields_no_session() {
        assert!(QuizSession::new(&quizzes(), Some(250), &mut Rng::new(3)).is_none());
    }

    #[test]
    fn abort_finishes_without_running_past_the_end() {
        let all = quizzes();
        let mut s = QuizSession::new(&all, None, &mut Rng::new(4)).unwrap();
        s.record(0, false);
        s.abort();
        assert!(s.finished && s.current().is_none());
        assert!(
            s.pos < s.order.len(),
            "регресс: раньше показывался «Вопрос {}/{}» сверх конца",
            s.pos + 1,
            s.order.len()
        );
        assert_eq!((s.answered, s.correct_count), (1, 0));
    }

    #[test]
    fn advancing_past_the_last_question_finishes() {
        let all = quizzes();
        let mut s = QuizSession::new(&all, Some(3), &mut Rng::new(5)).unwrap();
        for _ in 0..s.order.len() {
            assert!(s.current().is_some());
            s.advance();
        }
        assert!(s.finished && s.current().is_none());
    }

    #[test]
    fn methodology_flags_dynamic_before_triage_or_static() {
        let mut w = WorkSession::new();
        w.stages_done = vec!["triage".into(), "static".into(), "dynamic".into()];
        assert!(w.methodology_ok().is_ok());
        w.stages_done = vec!["dynamic".into(), "triage".into()];
        assert!(w.methodology_ok().is_err());
        w.stages_done = vec!["triage".into(), "dynamic".into(), "static".into()];
        assert!(w.methodology_ok().is_err());
    }

    #[test]
    fn report_completeness_counts_characters_not_bytes() {
        let mut w = WorkSession::new();
        assert_eq!(w.report_completeness(), 0);
        w.report[0] = "ELF64, gcc".into();
        w.report[1] = "хеш sha256".into(); // 10 символов кириллицей/латиницей = 15 байт
        w.report[2] = "коротко".into(); // 7 символов — не засчитывается
        assert_eq!(w.report_completeness(), 20);
    }
}
