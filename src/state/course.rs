//! Прогресс по курсу: недели, лабы, PSet, чек-пойнты, rubric.

use super::{xp, AppState};

impl AppState {
    pub fn toggle_week_done(&mut self, week_id: &str) {
        if !self.progress.weeks_done.remove(week_id) {
            self.progress.weeks_done.insert(week_id.to_string());
            self.award_once(&format!("week:{week_id}"), xp::WEEK);
        }
        self.mark_dirty();
        self.check_achievements();
    }

    /// PSet засчитывается вместе с объяснением «своими словами» (правило честности курса).
    pub fn complete_pset_with_explain(&mut self, key: &str, explain: String) {
        let text = explain.trim();
        if !text.is_empty() {
            self.progress
                .pset_explains
                .insert(key.to_string(), text.to_string());
            if self.progress.psets_done.insert(key.to_string()) {
                self.award_once(&format!("pset:{key}"), xp::PSET);
            }
            self.mark_dirty();
            self.check_achievements();
        }
        self.pset_pending_explain = None;
    }

    pub fn toggle_lab_step(&mut self, key: &str) {
        if !self.progress.lab_steps_done.remove(key) {
            self.progress.lab_steps_done.insert(key.to_string());
            self.award_once(&format!("lab:{key}"), xp::LAB_STEP);
        }
        self.mark_dirty();
        self.check_achievements();
    }

    pub fn toggle_rubric(&mut self, index: usize) {
        if !self.progress.rubric_done.remove(&index) {
            self.progress.rubric_done.insert(index);
            self.award_once(&format!("rubric:{index}"), xp::RUBRIC_ITEM);
        }
        self.mark_dirty();
    }

    pub fn reset_rubric(&mut self) {
        self.progress.rubric_done.clear();
        self.mark_dirty();
    }

    pub fn toggle_checkpoint(&mut self, week_id: &str, idx: usize) {
        let Some(n) = self
            .curriculum
            .week_by_id(week_id)
            .map(|w| w.checkpoint.len())
        else {
            return;
        };
        if idx >= n {
            return;
        }
        let items = self
            .progress
            .checkpoint
            .entry(week_id.to_string())
            .or_default();
        if items.len() < n {
            items.resize(n, false);
        }
        items[idx] = !items[idx];
        self.mark_dirty();
        self.check_achievements();
    }

    pub fn checkpoint_checked(&self, week_id: &str, idx: usize) -> bool {
        self.progress
            .checkpoint
            .get(week_id)
            .and_then(|v| v.get(idx))
            .copied()
            .unwrap_or(false)
    }

    /// Сданы все PSet недели (у недели без PSet — `false`).
    pub fn pset_done_for_week(&self, week_id: &str) -> bool {
        self.curriculum.week_by_id(week_id).is_some_and(|w| {
            !w.psets.is_empty()
                && (0..w.psets.len())
                    .all(|i| self.progress.psets_done.contains(&format!("{}:{i}", w.id)))
        })
    }

    pub fn psets_total(&self) -> usize {
        self.curriculum.weeks.iter().map(|w| w.psets.len()).sum()
    }

    pub fn psets_done_count(&self) -> usize {
        self.curriculum
            .weeks
            .iter()
            .map(|w| {
                (0..w.psets.len())
                    .filter(|i| self.progress.psets_done.contains(&format!("{}:{i}", w.id)))
                    .count()
            })
            .sum()
    }

    pub fn weeks_total(&self) -> usize {
        self.curriculum.weeks.len()
    }

    pub fn weeks_done_count(&self) -> usize {
        self.curriculum
            .weeks
            .iter()
            .filter(|w| self.progress.weeks_done.contains(&w.id))
            .count()
    }

    pub fn overall_percent(&self) -> f32 {
        let total = self.weeks_total() + self.psets_total();
        if total == 0 {
            return 0.0;
        }
        ((self.weeks_done_count() + self.psets_done_count()) as f32 / total as f32 * 100.0)
            .min(100.0)
    }

    /// Решённые дриллы категории (`asm`, `addr`, `pat`, `scr`).
    pub fn drills_solved_in(&self, prefix: &str) -> usize {
        self.progress
            .drills_solved
            .iter()
            .filter(|k| k.starts_with(prefix))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_a_week_off_and_on_pays_xp_once() {
        // регресс: раньше каждое включение давало +30 XP
        let mut app = AppState::in_memory();
        for _ in 0..4 {
            app.toggle_week_done("w0");
        }
        assert!(!app.progress.weeks_done.contains("w0"));
        app.toggle_week_done("w0");
        assert_eq!(app.progress.xp, xp::WEEK);
    }

    #[test]
    fn first_toggle_pays_and_repeat_does_not() {
        let mut app = AppState::in_memory();
        app.toggle_lab_step("w0:0");
        app.toggle_rubric(3);
        assert_eq!(app.progress.xp, xp::LAB_STEP + xp::RUBRIC_ITEM);
        app.toggle_lab_step("w0:0");
        app.toggle_lab_step("w0:0");
        app.toggle_rubric(3);
        app.toggle_rubric(3);
        assert_eq!(app.progress.xp, xp::LAB_STEP + xp::RUBRIC_ITEM);
        assert!(
            app.progress.lab_steps_done.contains("w0:0") && app.progress.rubric_done.contains(&3)
        );
    }

    #[test]
    fn rubric_does_not_pollute_lab_steps() {
        let mut app = AppState::in_memory();
        app.toggle_rubric(0);
        assert!(app.progress.lab_steps_done.is_empty());
        app.reset_rubric();
        assert!(app.progress.rubric_done.is_empty());
    }

    #[test]
    fn pset_needs_a_non_empty_explanation() {
        let mut app = AppState::in_memory();
        app.complete_pset_with_explain("w0:0", "   ".into());
        assert!(app.progress.psets_done.is_empty());
        app.complete_pset_with_explain("w0:0", " потому что ".into());
        app.complete_pset_with_explain("w0:0", "ещё раз".into());
        let bonus: u32 = app
            .curriculum
            .achievements
            .iter()
            .filter(|a| app.progress.achievements.contains(&a.id))
            .map(|a| a.xp)
            .sum();
        assert_eq!(
            app.progress.xp,
            xp::PSET + bonus,
            "PSet платит один раз, ачивка — своё"
        );
        assert_eq!(app.progress.pset_explains["w0:0"], "ещё раз");
        assert!(app.pset_done_for_week("w0"));
    }

    #[test]
    fn checkpoint_ignores_out_of_range_indexes() {
        let mut app = AppState::in_memory();
        app.toggle_checkpoint("нет-такой-недели", 0);
        app.toggle_checkpoint("w1-2", 999); // раньше — паника на индексации
        assert!(app.progress.checkpoint.is_empty());
        app.toggle_checkpoint("w1-2", 0);
        assert!(app.checkpoint_checked("w1-2", 0));
        app.toggle_checkpoint("w1-2", 0);
        assert!(!app.checkpoint_checked("w1-2", 0));
    }

    #[test]
    fn overall_percent_ignores_unknown_keys() {
        let mut app = AppState::in_memory();
        app.progress.psets_done.insert("удалённая-неделя:0".into());
        assert_eq!(app.psets_done_count(), 0);
        assert_eq!(app.overall_percent(), 0.0);
    }
}
