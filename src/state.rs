use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::curriculum::{Curriculum, Quiz};

// ---------- persistence ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Progress {
    /// week id -> done
    pub weeks_done: HashSet<String>,
    /// week id -> checklist item index -> checked
    pub checkpoint: HashMap<String, Vec<bool>>,
    /// "week:pset_index" -> done
    pub psets_done: HashSet<String>,
    /// lab steps: "week:step_index" -> done
    pub lab_steps_done: HashSet<String>,
    /// quiz id -> last answer index (None if unanswered)
    pub quiz_answers: HashMap<String, usize>,
    /// quiz ids answered correctly at least once
    pub quiz_correct: HashSet<String>,
    /// unlocked achievement ids
    pub achievements: HashSet<String>,
    pub journal: String,
    pub xp: u32,
    /// monthly re-certification: (дата, верных, всего)
    #[serde(default)]
    pub reexam_score: Option<(String, u32, u32)>,
    /// день (unix/день года) последнего запуска re-exam
    #[serde(default)]
    pub last_reexam_day: Option<u64>,
    #[serde(default)]
    pub card_levels: std::collections::HashMap<String, u8>,
    /// pset key -> student's self-explanation (rule of honesty)
    #[serde(default)]
    pub pset_explains: std::collections::HashMap<String, String>,
    /// solved challenge ids
    #[serde(default)]
    pub challenges_solved: std::collections::HashSet<String>,
    /// XP history: (unix_day, total_xp) — sampled on each save, capped at 400 points
    #[serde(default)]
    pub xp_history: Vec<(u64, u32)>,
    /// Решённые дриллы (id = "cat:idx"), персистентно
    #[serde(default)]
    pub drills_solved: std::collections::HashSet<String>,
    /// Время занятий: unix_day -> секунды в приложении (сэмпл при save)
    #[serde(default)]
    pub time_by_day: std::collections::HashMap<String, u64>,
    /// накопитель секунд с последнего save
    #[serde(default)]
    pub pending_seconds: u64,
    /// последний unix-секунд тика
    #[serde(default)]
    pub last_tick: u64,
    /// Настройки интерфейса: тема ("dark"/"light"), множитель шрифта (100..200)
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,
    /// Режим «Ставка»: challenge id -> гипотеза, написанная ДО решения
    #[serde(default)]
    pub challenge_bets: std::collections::HashMap<String, String>,
    /// Ставка оценена? (после решения студент отмечает: угадал механизм/нет)
    #[serde(default)]
    pub bet_results: std::collections::HashMap<String, bool>,
    /// current streak: (last_active_unix_day, streak_len)
    #[serde(default)]
    pub streak: (u64, u32),
}

impl Progress {
}

// ---------- quiz runner state (not persisted) ----------

pub struct QuizSession {
    pub order: Vec<usize>,          // indices into curriculum quizzes
    pub pos: usize,
    pub selected: Option<usize>,
    pub submitted: bool,
    pub correct_count: usize,
    pub answered: usize,
    pub filter_module: Option<u8>,
    pub finished: bool,
}

impl QuizSession {
    pub fn new(total: usize, filter_module: Option<u8>) -> Self {
        // deterministic shuffle by simple LCG so it varies per session but is reproducible enough
        let mut order: Vec<usize> = (0..total).collect();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(42);
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (x >> 33) as usize % (i + 1);
            order.swap(i, j);
        }
        Self {
            order,
            pos: 0,
            selected: None,
            submitted: false,
            correct_count: 0,
            answered: 0,
            filter_module,
            finished: false,
        }
    }
}

// ---------- app ----------

#[derive(Default, PartialEq, Clone, Copy)]
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

#[derive(Default)]
pub struct DrillState {
    pub which: usize,   // 0=asm 1=addr 2=pattern 3=script
    pub idx: usize,
    pub choice: Option<usize>,
    pub checked: bool,
    #[allow(dead_code)]
    pub input: String,
    pub show_answer: bool,
    pub solved: std::collections::HashSet<String>,
}

#[derive(Default)]
pub struct SimState {
    pub which: usize,            // 0=regs 1=pe 2=oep 3=generative
    pub gen_kind: usize,         // 0=rip 1=le 2=decode
    pub gen_seed: u64,
    pub gen_answer: String,
    pub gen_feedback: Option<(bool, String)>,
    pub task_idx: usize,
    pub show_answer: bool,
    pub user_input: std::collections::HashMap<String, String>, // regs task: reg->hex
    pub choice: Option<usize>,
    pub checked: bool,
    pub last_ok: bool,
    pub solved: std::collections::HashSet<String>, // task ids solved this session
}

pub struct AppState {
    pub curriculum: Curriculum,
    pub progress: Progress,
    pub tab: Tab,
    pub selected_week: usize,
    pub quiz: Option<QuizSession>,
    #[allow(dead_code)]
    pub quiz_feedback: Vec<(String, bool)>, // (quiz id, was correct) last session
    pub toast: Option<(String, f64)>,       // message, expiry (seconds since app start)
    pub start_time: f64,
    pub new_achievements: Vec<String>,
    pub cards: Option<CardSession>,
    pub placement: Option<PlacementState>,
    pub sim: SimState,
    pub drill: DrillState,
    pub progress_export_text: String,
    pub pset_pending_explain: Option<String>,
    pub challenge_input: std::collections::HashMap<String, String>,
    pub reexam: Option<ReexamState>,
    pub opponent: OpponentState,
    pub work: WorkSession,
    pub work_hypothesis_input: String,
    pub search_query: String,
    pub search_focus: bool,
    pub search_results: Vec<(String, String, String)>,
    #[allow(dead_code)]
    pub import_text: String,
}

/// Ежемесячная рекертификация: 10 случайных задач из пройденного материала.
pub struct ReexamState {
    pub questions: Vec<crate::curriculum::Quiz>,
    pub pos: usize,
    pub selected: Option<usize>,
    pub correct: u32,
    pub answered: bool,
    pub finished: bool,
}

/// Симулятор рабочей сессии аналитика: тикет -> методология -> отчёт.
#[derive(Default)]
pub struct WorkSession {
    pub active: bool,
    pub challenge_id: String,        // какой бинарь "пришёл" по тикету
    pub started_unix: u64,
    /// порядок действий: какие этапы закрыты и в каком порядке
    pub stages_done: Vec<String>,    // "triage","static","dynamic","report"
    pub hypotheses: Vec<String>,     // гипотезы как в «Ставке»
    pub triage_notes: String,        // заметки триажа
    pub static_notes: String,
    pub dynamic_notes: String,
    pub report: [String; 10],        // по rubric (10 пунктов)
    pub flag_found: bool,
    /// история закрытых тикетов: (id, секунд, гипотезы_верны, отчёт_полнота%)
    pub history: Vec<(String, u64, bool, u8)>,
}

impl WorkSession {
    pub fn new() -> Self { Default::default() }

    /// Порядок этапов корректен? (триаж раньше динамики, отчёт последним)
    pub fn methodology_ok(&self) -> Result<(), String> {
        let pos = |s: &str| self.stages_done.iter().position(|x| x == s);
        if let (Some(t), Some(d)) = (pos("triage"), pos("dynamic")) {
            if d < t {
                return Err("Динамика запущена ДО триажа. В реальной работе это риск: неизвестный сэмпл без первичной оценки = compro." .into());
            }
        }
        if let (Some(s), Some(d)) = (pos("static"), pos("dynamic")) {
            if d < s {
                return Err("Динамика до статики. Сначала гипотезы из статики, потом проверка — иначе ты не аналитик, а запускальщик.".into());
            }
        }
        Ok(())
    }

    pub fn report_completeness(&self) -> u8 {
        let filled = self.report.iter().filter(|s| s.trim().len() >= 10).count();
        (filled * 10) as u8
    }
}

/// Socratic-оппонент: вопрос, ответ студента, оценка, история.
#[derive(Default)]
pub struct OpponentState {
    pub q_index: usize,
    pub answer: String,
    pub verdict: Option<(u8, String)>, // (score, critique)
    pub llm_reply: Option<String>,
    pub llm_model: String,
    pub llm_in_flight: bool,
    pub best_scores: std::collections::HashMap<String, u8>,
}

pub struct PlacementState {
    pub pos: usize,
    pub score: usize,
    pub selected: Option<usize>,
    pub answered: bool,
}

pub struct CardSession {
    pub order: Vec<usize>,
    pub pos: usize,
    pub show_back: bool,
    pub again: Vec<usize>,
    pub known: Vec<usize>,
}

impl CardSession {
    pub fn new(total: usize) -> Self {
        let mut order: Vec<usize> = (0..total).collect();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(7);
        let mut x = seed | 1;
        for i in (1..order.len()).rev() {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (x >> 33) as usize % (i + 1);
            order.swap(i, j);
        }
        Self { order, pos: 0, show_back: false, again: Vec::new(), known: Vec::new() }
    }
}

fn default_theme() -> String { "dark".into() }
fn default_font_scale() -> f32 { 1.0 }

fn chrono_like_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86400;
    format!("day {days} (unix {})", secs)
}

impl AppState {
    /// Экспорт лабы недели: ~/re50-lab/week_<id>/TASK.md с лекциями, шагами, PSet, чекпоинтом.
    pub fn export_week_lab(&mut self, week_id: &str) -> Option<String> {
        let w = self.curriculum.weeks.iter().find(|w| w.id == week_id)?.clone();
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let dir = std::path::PathBuf::from(home).join("re50-lab").join(format!("week_{}", w.id));
        std::fs::create_dir_all(&dir).ok()?;
        let mut md = format!("# {} — {}\n\n", w.label(), w.title);
        md.push_str("## Лекции\n");
        for l in &w.lectures { md.push_str(&format!("- {l}\n")); }
        if let Some(lab) = &w.lab {
            md.push_str(&format!("\n## {} \n", lab.title));
            for (i, s) in lab.steps.iter().enumerate() { md.push_str(&format!("{}. {s}\n", i + 1)); }
        }
        md.push_str("\n## Problem Set\n");
        for p in &w.psets { md.push_str(&format!("- {p}\n")); }
        md.push_str("\n## Чекпоинт (самопроверка)\n");
        for c in &w.checkpoint { md.push_str(&format!("- [ ] {c}\n")); }
        if let Some(case) = &w.case {
            md.push_str(&format!("\n## Проблема недели\n{case}\n"));
        }
        let path = dir.join("TASK.md");
        std::fs::write(&path, md).ok()?;
        Some(dir.display().to_string())
    }

    /// Экспорт всего профиля в файл ~/re50-profile.json (для переноса на другую машину)
    pub fn export_profile_file(&mut self) -> Result<String, String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let path = std::path::PathBuf::from(home).join("re50-profile.json");
        let json = serde_json::to_string_pretty(&self.progress).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    }

    /// Импорт профиля из ~/re50-profile.json с перезаписью текущего прогресса.
    pub fn import_profile_file(&mut self) -> Result<String, String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let path = std::path::PathBuf::from(home).join("re50-profile.json");
        let data = std::fs::read_to_string(&path).map_err(|e| format!("Нет файла {path:?}: {e}"))?;
        let p: Progress = serde_json::from_str(&data).map_err(|e| format!("Повреждённый JSON: {e}"))?;
        self.progress = p;
        self.save();
        Ok(format!("Профиль импортирован из {path:?}"))
    }

    /// Экспорт журнала в ~/re50-journal.md
    pub fn export_journal(&mut self) -> Result<String, String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let path = std::path::PathBuf::from(home).join("re50-journal.md");
        let mut md = String::from("# RE-50 — Журнал обучения\n\n");
        md.push_str(&format!("Экспортирован: {}\n\n", chrono_like_date()));
        md.push_str(&format!("XP: {} | Недель закрыто: {}\n\n", self.progress.xp, self.progress.weeks_done.len()));
        md.push_str("## Журнал\n\n");
        md.push_str(&self.progress.journal);
        md.push_str("\n\n## Гипотезы (ставки)\n\n");
        for (id, bet) in &self.progress.challenge_bets {
            let ok = self.progress.bet_results.get(id).map(|b| if *b { "✔" } else { "✘" }).unwrap_or("⏳");
            md.push_str(&format!("- {ok} **{id}**: {bet}\n"));
        }
        md.push_str("\n## Тикеты (рабочие сессии)\n\n");
        for (id, dur, ok, comp) in &self.work.history {
            md.push_str(&format!("- {} {} — {} сек, отчёт {}%\n", if *ok { "✔" } else { "⚠" }, id, dur, comp));
        }
        std::fs::write(&path, md).map_err(|e| e.to_string())?;
        Ok(path.display().to_string())
    }

    /// Глобальный поиск по курсу: недели, квизы, дриллы, ресурсы, челленджи.
    /// Возвращает (заголовок результата, тип, week_id для перехода)
    pub fn search_course(&self, q: &str) -> Vec<(String, String, String)> {
        let q = q.trim().to_lowercase();
        if q.len() < 2 { return vec![]; }
        let mut out = Vec::new();
        for w in &self.curriculum.weeks {
            let hay = format!("{} {}", w.title, w.lectures.join(" ")).to_lowercase();
            if hay.contains(&q) {
                out.push((format!("📚 {}", w.title), "week".into(), w.id.clone()));
            }
            if let Some(lab) = &w.lab {
                let hay = format!("{} {}", lab.title, lab.steps.join(" ")).to_lowercase();
                if hay.contains(&q) {
                    out.push((format!("🧪 {} (лаба)", lab.title), "week".into(), w.id.clone()));
                }
            }
            for ps in &w.psets {
                if ps.to_lowercase().contains(&q) {
                    out.push((format!("✏️ {}", ps.chars().take(60).collect::<String>()), "week".into(), w.id.clone()));
                }
            }
        }
        for quiz in &self.curriculum.quizzes {
            if quiz.question.to_lowercase().contains(&q) {
                out.push((format!("❓ {}", quiz.question.chars().take(60).collect::<String>()), "quiz".into(), quiz._week.clone()));
            }
        }
        for ch in &self.curriculum.challenges {
            if format!("{} {}", ch.title, ch.desc).to_lowercase().contains(&q) {
                out.push((format!("🚩 {} ({})", ch.title, ch.id), "challenge".into(), String::new()));
            }
        }
        for r in &self.curriculum.resources {
            if format!("{} {}", r.name, r.category).to_lowercase().contains(&q) {
                out.push((format!("🔗 {} [{}]", r.name, r.category), "resource".into(), String::new()));
            }
        }
        out.truncate(30);
        out
    }

    /// Начать рабочую сессию: выбрать случайный нерешённый челлендж как "тикет".
    pub fn start_work_session(&mut self) {
        let mut candidates: Vec<String> = self
            .curriculum
            .challenges
            .iter()
            .filter(|ch| !self.progress.challenges_solved.contains(&ch.id))
            .map(|ch| ch.id.clone())
            .collect();
        if candidates.is_empty() {
            // всё решено — берём любой (повторная тренировка)
            candidates = self.curriculum.challenges.iter().map(|ch| ch.id.clone()).collect();
        }
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        let mut s = seed | 1;
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (s >> 33) as usize % candidates.len();
        let id = candidates[idx].clone();
        self.work = WorkSession {
            active: true,
            challenge_id: id,
            started_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            ..WorkSession::new()
        };
    }

    /// Закрыть тикет: время, методология, отчёт, XP.
    pub fn finish_work_session(&mut self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dur = now.saturating_sub(self.work.started_unix);
        let completeness = self.work.report_completeness();
        let method_err = self.work.methodology_ok().is_err();
        let hypotheses = self.work.hypotheses.len();
        let id = self.work.challenge_id.clone();
        let solved = self.progress.challenges_solved.contains(&id);
        self.progress.challenge_bets.insert(format!("work:{}", id), format!("гипотез: {hypotheses}"));
        self.work.history.push((id.clone(), dur, !method_err, completeness));
        let mut msg = format!("Тикет {id} закрыт за {dur} сек. Полнота отчёта: {completeness}%.");
        if method_err {
            msg.push_str(" ⚠ Нарушение методологии — см. замечание.");
        } else {
            msg.push_str(" Методология в порядке ✔");
        }
        let _ = solved;
        // XP: 30 за сессию + 20 за полноту отчёта >= 70%
        let xp = 30 + if completeness >= 70 { 20 } else { 0 };
        self.add_xp(xp);
        msg.push_str(&format!(" +{xp} XP"));
        self.work.active = false;
        msg
    }

    /// Прошло ли >=30 дней с последнего re-exam (или никогда не было).
    pub fn reexam_due(&self, day: u64) -> bool {
        match self.progress.last_reexam_day {
            Some(d) => day.saturating_sub(d) >= 30,
            None => self.progress.xp > 300, // имеет смысл после первой недели
        }
    }

    /// Запустить внезапный экзамен: 10 случайных квизов из пройденных недель.
    pub fn start_reexam(&mut self, seed: u64) {
        let done: Vec<&crate::curriculum::Quiz> = self
            .curriculum
            .quizzes
            .iter()
            .filter(|q| {
                self.progress
                    .quiz_correct
                    .contains(&q.id)
            })
            .collect();
        // Если мало пройденных — добираем случайными из всех
        let mut pool: Vec<crate::curriculum::Quiz> =
            if done.len() >= 10 { done.into_iter().cloned().collect() }
            else { self.curriculum.quizzes.clone() };
        // shuffle LCG with seed
        let mut s = seed | 1;
        for i in (1..pool.len()).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (s >> 33) as usize % (i + 1);
            pool.swap(i, j);
        }
        pool.truncate(10);
        self.reexam = Some(ReexamState {
            questions: pool,
            pos: 0,
            selected: None,
            correct: 0,
            answered: false,
            finished: false,
        });
    }

    pub fn finish_reexam(&mut self, date: String) {
        if let Some(rx) = &self.reexam {
            let (c, t) = (rx.correct, rx.questions.len() as u32);
            self.progress.reexam_score = Some((date, c, t));
            // ниже порога 70% -> соответствующие темы снова в слабые (сброс quiz_correct)
            if (c as f32) < 0.7 * (t as f32) {
                self.toast = Some((
                    format!("Re-certification: {c}/{t}. Порог 70% не пройден — перерешайте квизы слабых недель."),
                    8.0,
                ));
            } else {
                self.toast = Some((format!("Re-certification: {c}/{t} — порог пройден ✔"), 6.0));
            }
            let day = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            self.progress.last_reexam_day = Some(day);
        }
        if let Some(rx) = &mut self.reexam {
            rx.finished = true;
        }
    }
}

impl AppState {
    #[allow(dead_code)]
    pub fn with_curriculum(curriculum: Curriculum) -> Self {
        let progress = Progress::default();
        Self {
            curriculum,
            progress,
            tab: Tab::default(),
            selected_week: 0,
            quiz: None,
            quiz_feedback: Vec::new(),
            toast: None,
            start_time: 0.0,
            new_achievements: Vec::new(),
            cards: None,
            placement: None,
            sim: SimState::default(),
            drill: DrillState::default(),
            challenge_input: std::collections::HashMap::new(),
            reexam: None,
            opponent: OpponentState::default(),
            work: WorkSession::new(),
            work_hypothesis_input: String::new(),
            search_query: String::new(),
            search_focus: false,
            search_results: Vec::new(),
            import_text: String::new(),
            pset_pending_explain: None,
            progress_export_text: String::new(),
        }
    }

    /// Тестовый конструктор: без чтения/записи файлов на диске.
    #[cfg(test)]
    pub fn for_test() -> Self {
        let mut s = Self::load_or_default();
        s.progress = Progress::default();
        s
    }

    pub fn load_or_default() -> Self {
        let curriculum = Curriculum::load();
        let progress = Self::read_progress_file().unwrap_or_default();
        let drill = DrillState { solved: progress.drills_solved.clone(), ..DrillState::default() };
        Self {
            curriculum,
            progress,
            tab: Tab::default(),
            selected_week: 0,
            quiz: None,
            quiz_feedback: Vec::new(),
            toast: None,
            start_time: 0.0,
            new_achievements: Vec::new(),
            cards: None,
            placement: None,
            sim: SimState::default(),
            drill,
            progress_export_text: String::new(),
            pset_pending_explain: None,
            challenge_input: std::collections::HashMap::new(),
            reexam: None,
            opponent: OpponentState::default(),
            work: WorkSession::new(),
            work_hypothesis_input: String::new(),
            search_query: String::new(),
            search_focus: false,
            search_results: Vec::new(),
            import_text: String::new(),
        }
    }

    fn progress_path() -> Option<std::path::PathBuf> {
        dirs_next()
    }

    pub fn configure(&mut self, cc: &eframe::CreationContext<'_>) {
        self.start_time = 0.0; // egui i.time is seconds since app start
        egui_extras_note(cc);
        self.apply_style(&cc.egui_ctx);
    }

    /// Применить тему/шрифт/масштаб из настроек.
    pub fn apply_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let light = self.progress.theme == "light";
        style.visuals.dark_mode = !light;
        if light {
            style.visuals.panel_fill = egui::Color32::from_rgb(245, 245, 248);
            style.visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
            style.visuals.extreme_bg_color = egui::Color32::from_rgb(232, 232, 238);
        } else {
            style.visuals.panel_fill = egui::Color32::from_rgb(18, 20, 26);
            style.visuals.window_fill = egui::Color32::from_rgb(22, 24, 32);
            style.visuals.extreme_bg_color = egui::Color32::from_rgb(12, 13, 18);
        }
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(200, 40, 60);
        style.visuals.hyperlink_color = egui::Color32::from_rgb(255, 110, 110);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        // масштаб шрифта
        let scale = self.progress.font_scale.clamp(0.8, 2.0);
        for font_id in style.text_styles.values_mut() {
            font_id.size = (font_id.size * scale).clamp(8.0, 48.0);
        }
        ctx.set_style(style);
    }

    pub fn save(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let day = now / 86400;
        // учёт времени: дельта с прошлого сохранения, ограничена (anti-висение)
        let delta = self.progress.last_tick;
        let delta = if delta > 0 && now > delta { (now - delta).min(3600) } else { 0 };
        self.progress.pending_seconds += delta;
        self.progress.last_tick = now;
        if self.progress.pending_seconds >= 30 {
            let key = day.to_string();
            *self.progress.time_by_day.entry(key).or_insert(0) += self.progress.pending_seconds;
            self.progress.pending_seconds = 0;
        }
        // streak
        if self.progress.streak.0 == 0 {
            self.progress.streak = (day, 1);
        } else if day > self.progress.streak.0 {
            let delta = day - self.progress.streak.0;
            self.progress.streak.1 = if delta == 1 { self.progress.streak.1 + 1 } else { 1 };
            self.progress.streak.0 = day;
        }
        // history: one point per day
        let h = &mut self.progress.xp_history;
        match h.last_mut() {
            Some((d, xp)) if *d == day => *xp = self.progress.xp,
            _ => {
                h.push((day, self.progress.xp));
                if h.len() > 400 { h.remove(0); }
            }
        }
        if let Some(path) = Self::progress_path() {
            if let Ok(json) = serde_json::to_string_pretty(&self.progress) {
                let _ = std::fs::create_dir_all(path.parent().unwrap());
                // бэкап: ротация 5 копий — защита от порчи/случайной потери прогресса
                let dir = path.parent().unwrap().to_path_buf();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                for i in (1..5).rev() {
                    let from = dir.join(format!("{name}.bak{i}"));
                    let to = dir.join(format!("{name}.bak{}", i + 1));
                    if from.exists() { let _ = std::fs::rename(&from, &to); }
                }
                if path.exists() {
                    let _ = std::fs::copy(&path, dir.join(format!("{name}.bak1")));
                }
                let _ = std::fs::write(&path, json);
            }
        }
    }

    fn read_progress_file() -> Option<Progress> {
        let path = Self::progress_path()?;
        // основной файл, затем бэкапы bak1..bak5
        let dir = path.parent()?;
        let name = path.file_name()?.to_string_lossy().to_string();
        let mut candidates = vec![path.clone()];
        for i in 1..=5 {
            candidates.push(dir.join(format!("{name}.bak{i}")));
        }
        for p in candidates {
            if let Ok(data) = std::fs::read_to_string(&p) {
                if let Ok(progress) = serde_json::from_str::<Progress>(&data) {
                    if p != path {
                        eprintln!("re50: прогресс восстановлен из бэкапа {}", p.display());
                    }
                    return Some(progress);
                }
            }
        }
        None
    }

    pub fn now(&self, ctx: &egui::Context) -> f64 {
        ctx.input(|i| i.time)
    }

    pub fn toast(&mut self, msg: impl Into<String>, ctx: &egui::Context) {
        let until = self.now(ctx) + 3.0;
        self.toast = Some((msg.into(), until));
    }

    // ----- mutations + achievement checks -----

    pub fn add_xp(&mut self, xp: u32) {
        self.progress.xp += xp;
    }

    pub fn toggle_week_done(&mut self, week_id: &str) {
        if self.progress.weeks_done.contains(week_id) {
            self.progress.weeks_done.remove(week_id);
        } else {
            self.progress.weeks_done.insert(week_id.to_string());
            self.add_xp(30);
        }
        self.check_achievements();
    }

    pub fn complete_pset_with_explain(&mut self, key: &str, explain: String) {
        if !explain.trim().is_empty() {
            self.progress.pset_explains.insert(key.to_string(), explain.trim().to_string());
            if !self.progress.psets_done.contains(key) {
                self.progress.psets_done.insert(key.to_string());
                self.add_xp(40);
            }
            self.check_achievements();
        }
        self.pset_pending_explain = None;
    }

    #[allow(dead_code)]
    /// Export challenge binaries (ELF + EXE) to ~/re50-lab/<id>/
    pub fn export_challenge(&self, id: &str) -> Option<String> {
        let ch = self.curriculum.challenges.iter().find(|c| c.id == id)?;
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))?; // Windows
        let dir = std::path::PathBuf::from(home).join("re50-lab").join(id);
        std::fs::create_dir_all(&dir).ok()?;
        let elf_bytes = include_bytes_with_fallback(&format!("assets/challenges/{}", id));
        let exe_bytes = include_bytes_with_fallback(&format!("assets/challenges/{}.exe", id));
        if let Some(b) = elf_bytes {
            let p = dir.join(id);
            std::fs::write(&p, b).ok()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
            }
        }
        if let Some(b) = exe_bytes {
            let p = dir.join(format!("{id}.exe"));
            std::fs::write(&p, b).ok()?;
        }
        // write task note
        let note = format!(
            "Челлендж: {} (уровень {})\nЗадача: {}\nПодсказка: {}\n\nЗапуск:\n  Linux: ./{}\n  Windows: {}.exe\nРешите в Ghidra/x64dbg и введите флаг в приложении.",
            ch.title, ch.level, ch.desc, ch.hint, id, id
        );
        std::fs::write(dir.join("TASK.txt"), note).ok()?;
        Some(dir.to_string_lossy().to_string())
    }

    pub fn submit_flag(&mut self, id: &str, flag: &str) -> bool {
        let clean = flag.trim().trim_start_matches("FLAG{").trim_end_matches('}');
        let expected: String = self
            .curriculum
            .challenges
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.flag.clone())
            .unwrap_or_default();
        if clean == expected && !expected.is_empty() {
            if self.progress.challenges_solved.insert(id.to_string()) {
                self.add_xp(50);
            }
            self.check_achievements();
            true
        } else {
            false
        }
    }

    pub fn export_progress(&self) -> String {
        serde_json::to_string(&self.progress).unwrap_or_default()
    }

    pub fn import_progress(&mut self, json: &str) -> bool {
        match serde_json::from_str::<Progress>(json) {
            Ok(p) => {
                self.progress = p;
                self.check_achievements();
                true
            }
            Err(_) => false,
        }
    }

    /// three weakest quiz topics (module with most wrong answers not yet corrected)
    pub fn weak_topics(&self) -> Vec<(String, usize)> {
        let mut wrong: std::collections::HashMap<u8, usize> = std::collections::HashMap::new();
        for q in &self.curriculum.quizzes {
            let answered = self.progress.quiz_answers.get(&q.id);
            if let Some(&sel) = answered {
                if sel != q.correct && !self.progress.quiz_correct.contains(&q.id) {
                    *wrong.entry(q.module).or_insert(0) += 1;
                }
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

    #[allow(dead_code)]
    pub fn toggle_pset(&mut self, key: &str) {
        if self.progress.psets_done.contains(key) {
            self.progress.psets_done.remove(key);
        } else {
            self.progress.psets_done.insert(key.to_string());
            self.add_xp(40);
        }
        self.check_achievements();
    }

    pub fn toggle_checkpoint(&mut self, week_id: &str, idx: usize) {
        let n = self
            .curriculum
            .weeks
            .iter()
            .find(|w| w.id == week_id)
            .map(|w| w.checkpoint.len())
            .unwrap_or(0);
        let v = self
            .progress
            .checkpoint
            .entry(week_id.to_string())
            .or_insert(vec![false; n]);
        if v.len() < n {
            v.resize(n, false);
        }
        v[idx] = !v[idx];
        self.check_achievements();
    }

    pub fn toggle_lab_step(&mut self, key: &str) {
        if self.progress.lab_steps_done.contains(key) {
            self.progress.lab_steps_done.remove(key);
        } else {
            self.progress.lab_steps_done.insert(key.to_string());
            self.add_xp(10);
        }
        self.check_achievements();
    }

    pub fn submit_quiz_answer(&mut self, quiz: &Quiz, selected: usize) -> bool {
        let correct = selected == quiz.correct;
        self.progress.quiz_answers.insert(quiz.id.clone(), selected);
        if correct {
            self.progress.quiz_correct.insert(quiz.id.clone());
        }
        self.check_achievements();
        correct
    }

    pub fn psets_total(&self) -> usize {
        self.curriculum
            .weeks
            .iter()
            .map(|w| w.psets.len())
            .sum()
    }

    pub fn psets_done_count(&self) -> usize {
        self.progress.psets_done.len()
    }

    pub fn weeks_total(&self) -> usize {
        self.curriculum.weeks.len()
    }

    pub fn weeks_done_count(&self) -> usize {
        self.progress.weeks_done.len()
    }

    pub fn overall_percent(&self) -> f32 {
        let total = self.weeks_total() + self.psets_total();
        if total == 0 {
            return 0.0;
        }
        ((self.weeks_done_count() + self.psets_done_count()) as f32 / total as f32 * 100.0)
            .min(100.0)
    }

    pub fn check_achievements(&mut self) {
        let p = &self.progress;
        let has = |id: &str| p.achievements.contains(id);
        let mut unlock: Vec<&str> = Vec::new();

        let week_done = |id: &str| p.weeks_done.contains(id);
        let pset_done_prefix = |pre: &str| p.psets_done.iter().any(|k| k.starts_with(pre));

        if week_done("w0") && !has("start") {
            unlock.push("start");
        }
        if pset_done_prefix("w1-2:") && !has("c_master") {
            unlock.push("c_master");
        }
        if pset_done_prefix("w3-4:") && pset_done_prefix("w5:") && !has("asm_master") {
            unlock.push("asm_master");
        }
        if pset_done_prefix("w6:") && !has("pe_master") {
            unlock.push("pe_master");
        }
        if pset_done_prefix("w7:") && pset_done_prefix("w8:") && !has("ghidra_first") {
            unlock.push("ghidra_first");
        }
        if pset_done_prefix("w9:") && !has("triage") {
            unlock.push("triage");
        }
        // 20 crackmes: psets of weeks 10-14 (5 psets)
        let crack = ["w10-11:", "w12-13:", "w14:"]
            .iter()
            .filter(|pre| pset_done_prefix(pre))
            .count();
        if crack >= 3 && !has("crackmes_20") {
            unlock.push("crackmes_20");
        }
        if pset_done_prefix("w12-13:") && !has("keygen") {
            unlock.push("keygen");
        }
        if pset_done_prefix("w14:") && !has("antidebug") {
            unlock.push("antidebug");
        }
        if pset_done_prefix("w15-16:") && pset_done_prefix("w17:") && !has("pwn_college") {
            unlock.push("pwn_college");
        }
        if week_done("w18") && !has("lab_ready") {
            unlock.push("lab_ready");
        }
        if pset_done_prefix("w19-20:") && !has("full_pipeline") {
            unlock.push("full_pipeline");
        }
        if pset_done_prefix("w21:") && !has("ai_bridge") {
            unlock.push("ai_bridge");
        }
        let total_quizzes = self.curriculum.quizzes.len();
        if p.quiz_correct.len() * 10 >= total_quizzes * 9 && !has("quiz_90") {
            unlock.push("quiz_90");
        }
        let final_done = week_done("w23-24");
        if final_done && !has("final") {
            unlock.push("final");
        }

        // senior-track achievements
        if pset_done_prefix("w25:") && pset_done_prefix("w26:") && pset_done_prefix("w27:") && !has("elf_master") {
            unlock.push("elf_master");
        }
        if pset_done_prefix("w28:") && pset_done_prefix("w29:") && !has("cpp_master") {
            unlock.push("cpp_master");
        }
        if pset_done_prefix("w30:") && pset_done_prefix("w31:") && !has("net_master") {
            unlock.push("net_master");
        }
        if pset_done_prefix("w32:") && pset_done_prefix("w33:") && !has("auto_master") {
            unlock.push("auto_master");
        }
        if pset_done_prefix("w34:") && pset_done_prefix("w35:") && !has("mal_master") {
            unlock.push("mal_master");
        }
        if pset_done_prefix("w36:") && !has("senior") {
            unlock.push("senior");
        }

        for id in unlock {
            self.progress.achievements.insert(id.to_string());
            let (xp, name) = match self.curriculum.achievements.iter().find(|a| a.id == id) {
                Some(a) => (a.xp, a.name.clone()),
                None => (0, String::new()),
            };
            self.add_xp(xp);
            if !name.is_empty() {
                self.new_achievements.push(name);
            }
        }
    }
}

// eframe re-exports egui; keep a tiny shim so main.rs needs no direct egui dep line issues
pub use eframe::egui;

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::ui::run(self, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        AppState::save(self);
    }
}

// dirs shim: progress stored in user config dir
fn dirs_next() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|d| std::path::PathBuf::from(d).join("re50").join("progress.json"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|d| std::path::PathBuf::from(d).join("Library/Application Support/re50/progress.json"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
            .map(|d| d.join("re50").join("progress.json"))
    }
}

fn egui_extras_note(_cc: &eframe::CreationContext<'_>) {}

fn include_bytes_with_fallback(rel: &str) -> Option<&'static [u8]> {
    // Embedded at compile time via a generated file
    crate::challenge_blob::EMBEDDED_CHALLENGES
        .iter()
        .find(|(name, _)| *name == rel)
        .map(|(_, bytes)| *bytes)
}

