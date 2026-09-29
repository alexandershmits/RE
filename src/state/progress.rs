//! Персистентный прогресс студента и его миграции.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Версия схемы файла прогресса. 1 — до появления `xp_awarded`, `mistakes`, `rubric_done`.
pub const SCHEMA_VERSION: u32 = 2;
pub const MAX_XP_HISTORY: usize = 400;
pub const MIN_FONT_SCALE: f32 = 0.8;
pub const MAX_FONT_SCALE: f32 = 2.0;

/// Закрытый тикет рабочей сессии.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkRecord {
    pub challenge: String,
    pub seconds: u64,
    pub method_ok: bool,
    pub completeness: u8,
}

fn legacy_schema() -> u32 {
    1
}

/// Всё, что переживает перезапуск. `BTree*` — чтобы файл был детерминированным (чистые диффы бэкапов).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    /// Файл без этого поля — схема v1.
    #[serde(default = "legacy_schema")]
    pub schema: u32,
    pub weeks_done: BTreeSet<String>,
    /// week id -> отмеченные пункты чек-пойнта.
    pub checkpoint: BTreeMap<String, Vec<bool>>,
    /// `week:pset_index`
    pub psets_done: BTreeSet<String>,
    /// `week:step_index`
    pub lab_steps_done: BTreeSet<String>,
    /// Отмеченные пункты rubric отчёта.
    pub rubric_done: BTreeSet<usize>,
    pub quiz_answers: BTreeMap<String, usize>,
    /// Квизы, отвеченные верно хотя бы раз.
    pub quiz_correct: BTreeSet<String>,
    /// Квизы, в которых была ошибка и которые ещё не «выучены» карточками.
    pub mistakes: BTreeSet<String>,
    pub achievements: BTreeSet<String>,
    pub journal: String,
    pub xp: u32,
    /// Ключи уже выданного XP (`week:w1`, `lab:w1:0`, …): повтор не начисляется.
    pub xp_awarded: BTreeSet<String>,
    /// (дата, верных, всего)
    pub reexam_score: Option<(String, u32, u32)>,
    pub last_reexam_day: Option<u64>,
    /// Уровень карточки Лейтнера, 0..=5.
    pub card_levels: BTreeMap<String, u8>,
    /// День (с эпохи), когда карточка снова к повторению.
    pub card_due: BTreeMap<String, u64>,
    pub pset_explains: BTreeMap<String, String>,
    pub challenges_solved: BTreeSet<String>,
    /// (день, XP) — одна точка в день, не более `MAX_XP_HISTORY`.
    pub xp_history: Vec<(u64, u32)>,
    /// Решённые дриллы: `asm12`, `addr3`, …
    pub drills_solved: BTreeSet<String>,
    /// день -> секунды активной работы
    pub time_by_day: BTreeMap<String, u64>,
    pub pending_seconds: u64,
    /// Секунда последнего кадра; между запусками не используется.
    pub last_tick: u64,
    /// `dark` или `light`.
    pub theme: String,
    /// Множитель шрифта в пределах `MIN_FONT_SCALE..=MAX_FONT_SCALE`.
    pub font_scale: f32,
    /// Режим «Ставка»: challenge id -> гипотеза, написанная до решения.
    pub challenge_bets: BTreeMap<String, String>,
    pub bet_results: BTreeMap<String, bool>,
    /// (последний активный день, длина серии)
    pub streak: (u64, u32),
    /// Лучший балл Socratic-оппонента по теме.
    pub opponent_best: BTreeMap<String, u8>,
    pub work_history: Vec<WorkRecord>,
    /// (день, сколько задач генератора уже принесли XP в этот день)
    pub gen_practice: (u64, u32),
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            schema: SCHEMA_VERSION,
            weeks_done: BTreeSet::new(),
            checkpoint: BTreeMap::new(),
            psets_done: BTreeSet::new(),
            lab_steps_done: BTreeSet::new(),
            rubric_done: BTreeSet::new(),
            quiz_answers: BTreeMap::new(),
            quiz_correct: BTreeSet::new(),
            mistakes: BTreeSet::new(),
            achievements: BTreeSet::new(),
            journal: String::new(),
            xp: 0,
            xp_awarded: BTreeSet::new(),
            reexam_score: None,
            last_reexam_day: None,
            card_levels: BTreeMap::new(),
            card_due: BTreeMap::new(),
            pset_explains: BTreeMap::new(),
            challenges_solved: BTreeSet::new(),
            xp_history: Vec::new(),
            drills_solved: BTreeSet::new(),
            time_by_day: BTreeMap::new(),
            pending_seconds: 0,
            last_tick: 0,
            theme: "dark".into(),
            font_scale: 1.0,
            challenge_bets: BTreeMap::new(),
            bet_results: BTreeMap::new(),
            streak: (0, 0),
            opponent_best: BTreeMap::new(),
            work_history: Vec::new(),
            gen_practice: (0, 0),
        }
    }
}

impl Progress {
    pub fn is_light(&self) -> bool {
        self.theme == "light"
    }

    /// Приводит прочитанное/импортированное к допустимому виду и мигрирует старые схемы.
    pub fn sanitize(&mut self) {
        if self.schema < 2 {
            self.migrate_v1();
        }
        self.schema = SCHEMA_VERSION;
        if self.theme != "light" && self.theme != "dark" {
            self.theme = "dark".into();
        }
        self.font_scale = if self.font_scale.is_finite() && self.font_scale > 0.0 {
            self.font_scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE)
        } else {
            1.0
        };
        for level in self.card_levels.values_mut() {
            *level = (*level).min(5);
        }
        if self.xp_history.len() > MAX_XP_HISTORY {
            let excess = self.xp_history.len() - MAX_XP_HISTORY;
            self.xp_history.drain(..excess);
        }
    }

    fn migrate_v1(&mut self) {
        // rubric жил в lab_steps_done под ключами `rubric:N`
        let rubric: Vec<String> = self
            .lab_steps_done
            .iter()
            .filter(|k| k.starts_with("rubric:"))
            .cloned()
            .collect();
        for key in rubric {
            self.lab_steps_done.remove(&key);
            if let Ok(i) = key["rubric:".len()..].parse() {
                self.rubric_done.insert(i);
            }
        }
        // старые рабочие сессии писали в «ставки» служебные записи `work:<id>`: XP за них уже выдан
        let tickets: Vec<String> = self
            .challenge_bets
            .keys()
            .filter(|k| k.starts_with("work:"))
            .cloned()
            .collect();
        self.xp_awarded.extend(tickets);
        self.challenge_bets.retain(|k, _| !k.starts_with("work:"));
        // v1 не хранила ошибки отдельно: незакрытые = отвечены, но ни разу не верно
        let unsolved: Vec<String> = self
            .quiz_answers
            .keys()
            .filter(|id| !self.quiz_correct.contains(*id))
            .cloned()
            .collect();
        self.mistakes.extend(unsolved);
        // уже полученный XP нельзя получить повторно
        for id in &self.weeks_done {
            self.xp_awarded.insert(format!("week:{id}"));
        }
        for key in &self.lab_steps_done {
            self.xp_awarded.insert(format!("lab:{key}"));
        }
        for i in &self.rubric_done {
            self.xp_awarded.insert(format!("rubric:{i}"));
        }
        for key in &self.psets_done {
            self.xp_awarded.insert(format!("pset:{key}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_has_sane_settings() {
        let p = Progress::default();
        assert_eq!(
            (p.theme.as_str(), p.font_scale, p.schema),
            ("dark", 1.0, SCHEMA_VERSION)
        );
    }

    #[test]
    fn v1_file_is_migrated_without_double_paying_xp() {
        let v1 = r#"{
            "weeks_done": ["w0"], "psets_done": ["w0:0"], "xp": 100,
            "lab_steps_done": ["w0:0", "rubric:2"], "theme": "light", "font_scale": 0.0,
            "quiz_answers": {"q1": 0, "q2": 1}, "quiz_correct": ["q1"],
            "challenge_bets": {"work:lv1a": "гипотез: 2", "lv1b": "xor"}, "card_levels": {"quiz:q2": 255}
        }"#;
        let mut p: Progress = serde_json::from_str(v1).unwrap();
        assert_eq!(p.schema, 1, "файл без поля schema — это v1");
        p.sanitize();
        assert_eq!(p.schema, SCHEMA_VERSION);
        assert_eq!(p.rubric_done, BTreeSet::from([2]));
        assert_eq!(p.lab_steps_done, BTreeSet::from(["w0:0".to_string()]));
        for key in ["week:w0", "lab:w0:0", "rubric:2", "pset:w0:0"] {
            assert!(p.xp_awarded.contains(key), "{key}");
        }
        assert_eq!((p.xp, p.theme.as_str(), p.font_scale), (100, "light", 1.0));
        assert!(
            p.xp_awarded.contains("work:lv1a"),
            "закрытый тикет не должен заплатить второй раз"
        );
        assert_eq!(
            p.challenge_bets.keys().collect::<Vec<_>>(),
            ["lv1b"],
            "служебные «ставки» work:* убраны"
        );
        assert_eq!(
            p.mistakes,
            BTreeSet::from(["q2".to_string()]),
            "ошибки v1 становятся карточками"
        );
        assert_eq!(p.card_levels["quiz:q2"], 5, "уровень карточки ограничен");
    }

    #[test]
    fn sanitize_repairs_bad_settings() {
        let mut p = Progress {
            theme: "neon".into(),
            font_scale: f32::NAN,
            ..Progress::default()
        };
        p.sanitize();
        assert_eq!((p.theme.as_str(), p.font_scale), ("dark", 1.0));
        p.font_scale = 9.0;
        p.sanitize();
        assert_eq!(p.font_scale, MAX_FONT_SCALE);
        p.xp_history = (0..500).map(|i| (i, 0)).collect();
        p.sanitize();
        assert_eq!(
            (p.xp_history.len(), p.xp_history[0].0),
            (MAX_XP_HISTORY, 100)
        );
    }

    #[test]
    fn serialization_does_not_depend_on_insertion_order() {
        let (mut a, mut b) = (Progress::default(), Progress::default());
        for k in ["w3", "w1", "w2"] {
            a.weeks_done.insert(k.into());
        }
        for k in ["w2", "w3", "w1"] {
            b.weeks_done.insert(k.into());
        }
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    #[test]
    fn unknown_or_missing_fields_do_not_break_loading() {
        let p: Progress = serde_json::from_str(r#"{"xp": 5, "future_field": true}"#).unwrap();
        assert_eq!(p.xp, 5);
    }
}
