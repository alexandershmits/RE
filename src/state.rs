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

#[derive(Default, PartialEq)]
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
    pub which: usize,            // 0=regs 1=pe 2=oep
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
    #[allow(dead_code)]
    pub import_text: String,
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
            import_text: String::new(),
            pset_pending_explain: None,
            progress_export_text: String::new(),
        }
    }

    pub fn load_or_default() -> Self {
        let curriculum = Curriculum::load();
        let progress = Self::read_progress_file().unwrap_or_default();
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
            progress_export_text: String::new(),
            pset_pending_explain: None,
            challenge_input: std::collections::HashMap::new(),
            import_text: String::new(),
        }
    }

    fn progress_path() -> Option<std::path::PathBuf> {
        dirs_next()
    }

    pub fn configure(&mut self, cc: &eframe::CreationContext<'_>) {
        self.start_time = 0.0; // egui i.time is seconds since app start
        egui_extras_note(cc);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = egui::Color32::from_rgb(18, 20, 26);
        style.visuals.window_fill = egui::Color32::from_rgb(22, 24, 32);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(12, 13, 18);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(200, 40, 60);
        style.visuals.hyperlink_color = egui::Color32::from_rgb(255, 110, 110);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        cc.egui_ctx.set_style(style);
    }

    pub fn save(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let day = now / 86400;
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
                let _ = std::fs::write(&path, json);
            }
        }
    }

    fn read_progress_file() -> Option<Progress> {
        let path = Self::progress_path()?;
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
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

