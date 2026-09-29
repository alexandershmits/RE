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
/// Сколько результатов возвращает глобальный поиск.
pub const SEARCH_LIMIT: usize = 30;
/// Больше этого одна пауза между кадрами в учёт времени не идёт: ушёл, читал без ввода или закрыл крышку.
pub const IDLE_LIMIT_SECS: u64 = 300;

pub struct AppState {
    pub curriculum: Arc<Curriculum>,
    pub progress: Progress,
    pub paths: Paths,
    storage: Storage,
    dirty: bool,
    last_flush: Option<Instant>,
    /// Не чаще одной записи за этот интервал (в тестах подменяется).
    pub(crate) flush_interval: Duration,
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
    /// Момент, когда всплывающую ачивку пора скрыть (часы egui).
    pub popup_until: Option<f64>,
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
        util::use_local_timezone();
        Self::with_paths(Paths::detect())
    }

    pub fn with_paths(paths: Paths) -> Self {
        let no_config_dir = paths.config_dir.is_none();
        let mut app = Self::build(paths, Arc::new(Curriculum::load()), util::unix_now());
        if no_config_dir {
            app.startup_notice = Some(
                "Не удалось определить папку для настроек: прогресс не будет сохраняться. \
                 Задайте переменную RE50_CONFIG_DIR."
                    .into(),
            );
        }
        app
    }

    /// Без диска: ничего не читается и не пишется (тесты, превью).
    pub fn in_memory() -> Self {
        Self::build(
            Paths::none(),
            Arc::new(Curriculum::load()),
            util::unix_now(),
        )
    }

    pub fn with_curriculum(curriculum: Curriculum) -> Self {
        Self::build(Paths::none(), Arc::new(curriculum), util::unix_now())
    }

    pub(crate) fn build(paths: Paths, curriculum: Arc<Curriculum>, now: u64) -> Self {
        let mut storage = Storage::new(paths.progress_file());
        let loaded = storage.load::<Progress>();
        let mut notice = loaded.notice;
        let mut progress = loaded.value.unwrap_or_default();
        if progress.schema > progress::SCHEMA_VERSION {
            storage.disable_saving("файл прогресса создан более новой версией RE-50");
            let note = format!(
                "Файл прогресса создан более новой версией RE-50 (схема {}, эта версия понимает {}). \
                 Обновите приложение: пока сохранение отключено, чтобы ничего не потерять.",
                progress.schema,
                progress::SCHEMA_VERSION
            );
            notice = Some(match notice {
                Some(n) => format!("{n}. {note}"),
                None => note,
            });
        }
        progress.sanitize();
        // пауза между запусками не должна попадать в учёт времени
        progress.last_tick = 0;
        let today = util::local_day(now);
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
            dirty: loaded.needs_save,
            last_flush: None,
            flush_interval: FLUSH_INTERVAL,
            saves_written: 0,
            save_error: None,
            startup_notice: notice,
            days_away,
            tab: Tab::default(),
            selected_week: 0,
            quiz: None,
            toast: None,
            new_achievements: Vec::new(),
            popup_until: None,
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
        if p.last_tick != 0 && now > p.last_tick {
            p.pending_seconds += (now - p.last_tick).min(IDLE_LIMIT_SECS);
        }
        p.last_tick = now;
        let day = util::local_day(now);
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
                .is_some_and(|t| t.elapsed() < self.flush_interval)
        {
            return;
        }
        self.record_xp_sample(util::local_day(util::unix_now()));
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
            Some(t) => self.flush_interval.saturating_sub(t.elapsed()),
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

    /// Глобальный поиск: недели, лабы, PSet, квизы, челленджи, ресурсы (не более `SEARCH_LIMIT` результатов).
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
        out.truncate(SEARCH_LIMIT);
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
    fn active_time_is_capped_per_gap() {
        let mut app = AppState::in_memory();
        let t0 = 1_000_000;
        let total = |a: &AppState| {
            a.progress.time_by_day.values().sum::<u64>() + a.progress.pending_seconds
        };
        app.tick(t0); // первый кадр: точки отсчёта ещё нет
        assert_eq!(total(&app), 0);
        app.tick(t0 + 20);
        assert_eq!(total(&app), 20);
        app.tick(t0 + 20 + IDLE_LIMIT_SECS - 1); // читал без ввода почти до предела — засчитано целиком
        assert_eq!(total(&app), 20 + IDLE_LIMIT_SECS - 1);
        app.tick(t0 + 20 + IDLE_LIMIT_SECS - 1 + 3_600); // ушёл на час — не больше предела
        assert_eq!(total(&app), 20 + 2 * IDLE_LIMIT_SECS - 1);
        let day = util::local_day(t0).to_string();
        assert_eq!(app.progress.time_by_day[&day], 20 + 2 * IDLE_LIMIT_SECS - 1);
    }

    #[test]
    fn gap_between_launches_is_not_phantom_study_time() {
        // регресс: раньше при старте «через сутки» приложение приписывало до часа учёбы
        let dir = TempDir::new("phantom");
        let mut first = app_in(&dir, 1_000_000);
        first.tick(1_000_000);
        first.tick(1_000_010);
        first.flush(true);
        drop(first);
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
        app.flush_interval = Duration::from_secs(3600); // на загруженной машине «2 секунды» могли бы пройти
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
        drop(a);
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
        drop(a);
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
        drop(a);
        assert_eq!(app_in(&dir, 14 * 86_400).days_away, 4);
        assert_eq!(AppState::in_memory().days_away, 0);
    }

    #[test]
    fn second_window_keeps_the_first_windows_progress() {
        let dir = TempDir::new("twowindows");
        let mut first = app_in(&dir, 1_000_000);
        first.add_xp(50);
        first.flush(true);
        let mut second = app_in(&dir, 1_000_000);
        assert_eq!(second.progress.xp, 50, "второе окно видит прогресс");
        assert!(second
            .startup_notice
            .as_ref()
            .is_some_and(|n| n.contains("другом окне")));
        assert!(!second.is_dirty(), "предупреждение не повод писать на диск");
        second.add_xp(1_000);
        second.flush(true);
        assert!(
            second.save_error.is_some(),
            "пользователь должен увидеть, что запись отключена"
        );
        assert_eq!(second.saves_written, 0);
        first.add_xp(5);
        first.flush(true);
        drop((first, second));
        assert_eq!(app_in(&dir, 1_000_000).progress.xp, 55);
    }

    #[test]
    fn file_from_a_newer_version_is_never_overwritten() {
        let dir = TempDir::new("newer");
        let file = dir.path().join("config").join("progress.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let future = r#"{"schema": 99, "xp": 500, "from_the_future": [1, 2, 3]}"#;
        std::fs::write(&file, future).unwrap();
        let mut app = app_in(&dir, 1_000_000);
        assert_eq!(app.progress.xp, 500);
        assert!(app
            .startup_notice
            .as_ref()
            .is_some_and(|n| n.contains("более новой версией") && n.contains("99")));
        app.add_xp(1);
        app.flush(true);
        assert!(app.save_error.is_some());
        drop(app);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), future);
    }

    #[test]
    fn missing_config_dir_is_reported_but_in_memory_is_quiet() {
        let app = AppState::with_paths(Paths::none());
        assert!(app
            .startup_notice
            .as_ref()
            .is_some_and(|n| n.contains("RE50_CONFIG_DIR")));
        assert!(AppState::in_memory().startup_notice.is_none());
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
        let titles = |q: &str| {
            app.search_course(q)
                .into_iter()
                .map(|h| h.title)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            titles("  ghidra  "),
            titles("ghidra"),
            "пробелы по краям не влияют"
        );
        assert_eq!(
            titles("функци").len(),
            SEARCH_LIMIT,
            "частое слово упирается в потолок выдачи"
        );
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
