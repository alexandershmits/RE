//! Состояние приложения: прогресс, интерактивные сессии, сохранение.

pub mod achievements;
mod course;
mod exports;
mod practice;
pub mod progress;
pub mod sessions;

pub use practice::{
    normalize_flag, CARD_INTERVAL_DAYS, MISTAKE_GRADUATION_LEVEL, REEXAM_PASS_PERCENT,
    REEXAM_QUESTIONS, WORK_MIN_COMPLETENESS,
};
pub use progress::{Progress, WorkRecord};
pub use sessions::*;

use crate::curriculum::Curriculum;
use crate::jobs::Job;
use crate::storage::{Paths, Storage};
use crate::util;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Награды в XP.
pub mod xp {
    pub const WEEK: u32 = 30;
    pub const LAB_STEP: u32 = 10;
    pub const RUBRIC_ITEM: u32 = 10;
    pub const PSET: u32 = 40;
    pub const FLAG: u32 = 50;
    pub const DRILL: u32 = 10;
    pub const SIM_REGS: u32 = 15;
    pub const SIM_PE: u32 = 15;
    pub const SIM_OEP: u32 = 20;
    pub const GENERATED: u32 = 10;
    /// Сколько задач бесконечного генератора в день приносят XP.
    pub const GENERATED_PER_DAY: u32 = 10;
    pub const WORK_BASE: u32 = 30;
    pub const WORK_BONUS: u32 = 20;
}

/// Не чаще одной записи на диск за этот интервал.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
/// Пауза между кадрами длиннее этой считается отсутствием и не идёт в учёт времени.
pub const IDLE_LIMIT_SECS: u64 = 300;

pub struct AppState {
    pub curriculum: Arc<Curriculum>,
    pub progress: Progress,
    pub paths: Paths,
    storage: Storage,
    dirty: bool,
    last_flush: Option<Instant>,
    /// Сколько раз прогресс реально записан на диск.
    pub saves_written: u32,
    pub save_error: Option<String>,
    /// Сообщение о восстановлении прогресса при запуске.
    pub startup_notice: Option<String>,
    /// Дней без занятий на момент запуска.
    pub days_away: u64,
    pub tab: Tab,
    pub selected_week: usize,
    pub quiz: Option<QuizSession>,
    /// (текст, момент исчезновения по часам egui)
    pub toast: Option<(String, f64)>,
    pub new_achievements: Vec<String>,
    pub cards: Option<CardSession>,
    pub placement: Option<PlacementState>,
    pub sim: SimState,
    pub drill: DrillState,
    pub progress_export_text: String,
    pub pset_pending_explain: Option<String>,
    /// Текст объяснения в окне «Правило честности».
    pub pset_draft: String,
    pub challenge_input: BTreeMap<String, String>,
    pub reexam: Option<ReexamState>,
    pub opponent: OpponentState,
    pub work: WorkSession,
    pub work_hypothesis_input: String,
    pub search_query: String,
    pub search_focus: bool,
    pub search_results: Vec<SearchHit>,
    /// `Some(текст)` — открыто окно импорта прогресса.
    pub import_dialog: Option<String>,
    pub generator: Option<Job<GeneratorReport>>,
}

impl AppState {
    /// Прогресс читается из платформенного каталога настроек.
    pub fn load_or_default() -> Self {
        Self::with_paths(Paths::detect())
    }

    pub fn with_paths(paths: Paths) -> Self {
        Self::build(paths, Arc::new(Curriculum::load()), util::unix_now())
    }

    /// Без диска: ничего не читается и не пишется (тесты, превью).
    pub fn in_memory() -> Self {
        Self::with_paths(Paths::none())
    }

    pub fn with_curriculum(curriculum: Curriculum) -> Self {
        Self::build(Paths::none(), Arc::new(curriculum), util::unix_now())
    }

    pub(crate) fn build(paths: Paths, curriculum: Arc<Curriculum>, now: u64) -> Self {
        let mut storage = Storage::new(paths.progress_file());
        let loaded = storage.load::<Progress>();
        let restored = loaded.notice.is_some();
        let mut progress = loaded.value.unwrap_or_default();
        progress.sanitize();
        // пауза между запусками не должна попадать в учёт времени
        progress.last_tick = 0;
        let today = util::unix_day(now);
        let days_away = if progress.streak.0 > 0 {
            today.saturating_sub(progress.streak.0)
        } else {
            0
        };
        AppState {
            curriculum,
            progress,
            paths,
            storage,
            dirty: restored,
            last_flush: None,
            saves_written: 0,
            save_error: None,
            startup_notice: loaded.notice,
            days_away,
            tab: Tab::default(),
            selected_week: 0,
            quiz: None,
            toast: None,
            new_achievements: Vec::new(),
            cards: None,
            placement: None,
            sim: SimState::default(),
            drill: DrillState::default(),
            progress_export_text: String::new(),
            pset_pending_explain: None,
            pset_draft: String::new(),
            challenge_input: BTreeMap::new(),
            reexam: None,
            opponent: OpponentState::default(),
            work: WorkSession::new(),
            work_hypothesis_input: String::new(),
            search_query: String::new(),
            search_focus: false,
            search_results: Vec::new(),
            import_dialog: None,
            generator: None,
        }
    }

    pub fn configure(&mut self, cc: &eframe::CreationContext<'_>) {
        crate::ui::install(&cc.egui_ctx, &self.progress);
    }

    // ── сохранение и время ──

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Вызывается каждый кадр: учитывает активное время и серию дней.
    pub fn tick(&mut self, now: u64) {
        let p = &mut self.progress;
        if p.last_tick != 0 && now > p.last_tick && now - p.last_tick <= IDLE_LIMIT_SECS {
            p.pending_seconds += now - p.last_tick;
        }
        p.last_tick = now;
        let day = util::unix_day(now);
        if p.pending_seconds >= 30 {
            *p.time_by_day.entry(day.to_string()).or_insert(0) += p.pending_seconds;
            p.pending_seconds = 0;
            self.dirty = true;
        }
        let (last, len) = p.streak;
        if last == 0 {
            p.streak = (day, 1);
            self.dirty = true;
        } else if day > last {
            p.streak = (day, if day - last == 1 { len + 1 } else { 1 });
            self.dirty = true;
        }
    }

    /// Длина серии на сегодня: пропущенный день обнуляет её.
    pub fn streak_days(&self, today: u64) -> u32 {
        let (last, len) = self.progress.streak;
        if last != 0 && today.saturating_sub(last) <= 1 {
            len
        } else {
            0
        }
    }

    /// Записывает прогресс, если он изменился. Не чаще `FLUSH_INTERVAL`, кроме `force`.
    pub fn flush(&mut self, force: bool) {
        if !self.dirty {
            return;
        }
        if !force
            && self
                .last_flush
                .is_some_and(|t| t.elapsed() < FLUSH_INTERVAL)
        {
            return;
        }
        self.record_xp_sample(util::unix_day(util::unix_now()));
        match self.storage.save(&self.progress) {
            Ok(written) => {
                self.saves_written += u32::from(written);
                self.dirty = false;
                self.save_error = None;
            }
            Err(e) => self.save_error = Some(e),
        }
        self.last_flush = Some(Instant::now());
    }

    /// Через сколько нужен ещё один кадр, чтобы записать накопленные изменения.
    pub fn flush_due_in(&self) -> Option<Duration> {
        if !self.dirty {
            return None;
        }
        Some(match self.last_flush {
            Some(t) => FLUSH_INTERVAL.saturating_sub(t.elapsed()),
            None => Duration::ZERO,
        })
    }

    fn record_xp_sample(&mut self, day: u64) {
        let xp = self.progress.xp;
        match self.progress.xp_history.last_mut() {
            Some((d, value)) if *d == day => *value = xp,
            _ => {
                self.progress.xp_history.push((day, xp));
                if self.progress.xp_history.len() > progress::MAX_XP_HISTORY {
                    self.progress.xp_history.remove(0);
                }
            }
        }
    }

    // ── XP ──

    pub fn add_xp(&mut self, amount: u32) {
        self.progress.xp = self.progress.xp.saturating_add(amount);
        self.mark_dirty();
    }

    /// Начисляет XP один раз на ключ. `false` — награда по этому ключу уже выдана.
    pub fn award_once(&mut self, key: &str, amount: u32) -> bool {
        if self.progress.xp_awarded.insert(key.to_string()) {
            self.add_xp(amount);
            true
        } else {
            false
        }
    }

    // ── уведомления ──

    pub fn now(&self, ctx: &egui::Context) -> f64 {
        ctx.input(|i| i.time)
    }

    pub fn toast(&mut self, msg: impl Into<String>, ctx: &egui::Context) {
        self.toast_for(msg, ctx, 3.0);
    }

    pub fn toast_for(&mut self, msg: impl Into<String>, ctx: &egui::Context, secs: f64) {
        self.toast = Some((msg.into(), self.now(ctx) + secs));
    }

    // ── поиск ──

    /// Глобальный поиск: недели, лабы, PSet, квизы, челленджи, ресурсы (не более 30 результатов).
    pub fn search_course(&self, query: &str) -> Vec<SearchHit> {
        let q = query.trim().to_lowercase();
        if q.chars().count() < 2 {
            return Vec::new();
        }
        let c = &*self.curriculum;
        let mut out = Vec::new();
        let mut hit = |title: String, kind: HitKind, week_id: &str| {
            out.push(SearchHit {
                title,
                kind,
                week_id: week_id.to_string(),
            });
        };
        for w in &c.weeks {
            if format!("{} {}", w.title, w.lectures.join(" "))
                .to_lowercase()
                .contains(&q)
            {
                hit(format!("📚 {}", w.title), HitKind::Week, &w.id);
            }
            if let Some(lab) = &w.lab {
                if format!("{} {}", lab.title, lab.steps.join(" "))
                    .to_lowercase()
                    .contains(&q)
                {
                    hit(format!("🧪 {} (лаба)", lab.title), HitKind::Week, &w.id);
                }
            }
            for ps in w.psets.iter().filter(|p| p.to_lowercase().contains(&q)) {
                hit(
                    format!("✏ {}", ps.chars().take(60).collect::<String>()),
                    HitKind::Week,
                    &w.id,
                );
            }
        }
        for quiz in c
            .quizzes
            .iter()
            .filter(|z| z.question.to_lowercase().contains(&q))
        {
            hit(
                format!("❓ {}", quiz.question.chars().take(60).collect::<String>()),
                HitKind::Quiz,
                &quiz.week,
            );
        }
        for ch in &c.challenges {
            if format!("{} {}", ch.title, ch.desc)
                .to_lowercase()
                .contains(&q)
            {
                hit(
                    format!("🚩 {} ({})", ch.title, ch.id),
                    HitKind::Challenge,
                    "",
                );
            }
        }
        for r in &c.resources {
            if format!("{} {}", r.name, r.category)
                .to_lowercase()
                .contains(&q)
            {
                hit(
                    format!("🔗 {} [{}]", r.name, r.category),
                    HitKind::Resource,
                    "",
                );
            }
        }
        out.truncate(30);
        out
    }
}

impl eframe::App for AppState {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.tick(util::unix_now());
        crate::ui::run(self, ui);
        self.flush(false);
        if let Some(wait) = self.flush_due_in() {
            ui.ctx().request_repaint_after(wait);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.tick(util::unix_now());
        self.flush(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn app_in(dir: &TempDir, now: u64) -> AppState {
        AppState::build(Paths::in_dir(dir.path()), Arc::new(Curriculum::load()), now)
    }

    #[test]
    fn active_time_counts_short_gaps_only() {
        let mut app = AppState::in_memory();
        let t0 = 1_000_000;
        app.tick(t0); // первый кадр: точки отсчёта ещё нет
        assert_eq!(app.progress.pending_seconds, 0);
        app.tick(t0 + 20);
        assert_eq!(app.progress.pending_seconds, 20);
        app.tick(t0 + 20 + IDLE_LIMIT_SECS + 1); // ушёл и вернулся — паузу не считаем
        assert_eq!(app.progress.pending_seconds, 20);
        app.tick(t0 + 20 + IDLE_LIMIT_SECS + 11);
        assert_eq!(app.progress.pending_seconds, 30 - 30); // накоплено 30 → перенесено в сутки
        let day = util::unix_day(t0 + 20 + IDLE_LIMIT_SECS + 11).to_string();
        assert_eq!(app.progress.time_by_day[&day], 30);
    }

    #[test]
    fn gap_between_launches_is_not_phantom_study_time() {
        // регресс: раньше при старте «через сутки» приложение приписывало до часа учёбы
        let dir = TempDir::new("phantom");
        let mut first = app_in(&dir, 1_000_000);
        first.tick(1_000_000);
        first.tick(1_000_010);
        first.flush(true);
        let mut second = app_in(&dir, 1_000_000 + 86_400);
        second.tick(1_000_000 + 86_400);
        assert_eq!(
            second.progress.pending_seconds, 10,
            "остаток прошлого сеанса сохраняется, сутки паузы — нет"
        );
        assert_eq!(second.progress.time_by_day.values().sum::<u64>(), 0);
    }

    #[test]
    fn streak_grows_daily_and_breaks_after_a_gap() {
        let mut app = AppState::in_memory();
        let d = 86_400;
        app.tick(10 * d);
        assert_eq!(app.progress.streak, (10, 1));
        app.tick(10 * d + 5);
        assert_eq!(
            app.progress.streak,
            (10, 1),
            "второй кадр того же дня серию не растит"
        );
        app.tick(11 * d);
        assert_eq!(app.progress.streak, (11, 2));
        assert_eq!(app.streak_days(11), 2);
        assert_eq!(
            app.streak_days(12),
            2,
            "вчерашняя серия ещё жива до конца сегодняшнего дня"
        );
        assert_eq!(app.streak_days(13), 0, "пропущенный день обнуляет серию");
        app.tick(14 * d);
        assert_eq!(app.progress.streak, (14, 1));
    }

    #[test]
    fn flush_writes_only_when_dirty_and_content_changed() {
        let dir = TempDir::new("flush");
        let mut app = app_in(&dir, 1_000_000);
        app.flush(true);
        assert_eq!(app.saves_written, 0, "нечего сохранять");
        app.add_xp(5);
        app.flush(true);
        assert_eq!(app.saves_written, 1);
        app.flush(true);
        assert_eq!(app.saves_written, 1, "без изменений диск не трогаем");
        app.mark_dirty();
        app.flush(true);
        assert_eq!(app.saves_written, 1, "то же содержимое не перезаписывается");
    }

    #[test]
    fn flush_is_rate_limited_unless_forced() {
        let dir = TempDir::new("ratelimit");
        let mut app = app_in(&dir, 1_000_000);
        app.add_xp(1);
        app.flush(false);
        app.add_xp(1);
        app.flush(false);
        assert_eq!(app.saves_written, 1);
        assert!(
            app.flush_due_in().is_some(),
            "изменения ждут следующего кадра"
        );
        app.flush(true);
        assert_eq!(app.saves_written, 2);
        assert!(app.flush_due_in().is_none());
    }

    #[test]
    fn progress_survives_restart() {
        let dir = TempDir::new("restart");
        let mut a = app_in(&dir, 1_000_000);
        a.add_xp(123);
        a.progress.journal = "заметка".into();
        a.flush(true);
        let b = app_in(&dir, 1_000_000);
        assert_eq!(
            (b.progress.xp, b.progress.journal.as_str()),
            (123, "заметка")
        );
        assert!(b.startup_notice.is_none());
    }

    #[test]
    fn corrupted_progress_is_recovered_and_reported() {
        let dir = TempDir::new("recover");
        let mut a = app_in(&dir, 1_000_000);
        a.add_xp(77);
        a.flush(true);
        drop(app_in(&dir, 1_000_000)); // ротация: bak1 = сохранённый прогресс
        let file = dir.path().join("config").join("progress.json");
        std::fs::write(&file, "мусор").unwrap();
        let b = app_in(&dir, 1_000_000);
        assert_eq!(b.progress.xp, 77);
        assert!(b
            .startup_notice
            .as_ref()
            .is_some_and(|n| n.contains("резервной")));
        assert!(
            b.is_dirty(),
            "восстановленное состояние надо записать обратно"
        );
    }

    #[test]
    fn days_away_is_measured_at_launch() {
        let dir = TempDir::new("away");
        let mut a = app_in(&dir, 10 * 86_400);
        a.tick(10 * 86_400);
        a.flush(true);
        assert_eq!(app_in(&dir, 14 * 86_400).days_away, 4);
        assert_eq!(AppState::in_memory().days_away, 0);
    }

    #[test]
    fn award_once_pays_a_key_only_once() {
        let mut app = AppState::in_memory();
        assert!(app.award_once("k", 30));
        assert!(!app.award_once("k", 30));
        assert_eq!(app.progress.xp, 30);
        app.progress.xp = u32::MAX - 1;
        app.add_xp(50);
        assert_eq!(
            app.progress.xp,
            u32::MAX,
            "переполнение XP не должно паниковать"
        );
    }

    #[test]
    fn search_finds_content_and_ignores_short_queries() {
        let app = AppState::in_memory();
        assert!(!app.search_course("Ghidra").is_empty());
        assert!(app.search_course("x").is_empty());
        assert!(
            app.search_course("я").is_empty(),
            "один символ — слишком коротко, даже кириллический"
        );
        assert!(app.search_course("несуществующееслово123").is_empty());
        assert!(app.search_course("  ghidra  ").len() <= 30);
    }

    #[test]
    fn xp_history_keeps_one_point_per_day() {
        let mut app = AppState::in_memory();
        app.record_xp_sample(5);
        app.progress.xp = 40;
        app.record_xp_sample(5);
        app.record_xp_sample(6);
        assert_eq!(app.progress.xp_history, vec![(5, 40), (6, 40)]);
    }
}
