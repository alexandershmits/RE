//! Ачивки: условия описаны данными (`Rule` в curriculum.json), а не кодом.

use super::AppState;
use crate::curriculum::Rule;

impl AppState {
    /// Все квизы модуля решены верно (в модуле без квизов — `false`).
    fn module_quizzes_mastered(&self, module: u8) -> bool {
        let mut quizzes = self
            .curriculum
            .quizzes
            .iter()
            .filter(|q| q.module == module)
            .peekable();
        quizzes.peek().is_some() && quizzes.all(|q| self.progress.quiz_correct.contains(&q.id))
    }

    /// Сколько квизов курса решено верно (устаревшие id не считаются).
    pub fn quiz_correct_count(&self) -> usize {
        self.curriculum
            .quizzes
            .iter()
            .filter(|q| self.progress.quiz_correct.contains(&q.id))
            .count()
    }

    pub fn achievement_met(&self, rule: &Rule) -> bool {
        if rule.is_empty() {
            return false;
        }
        let quizzes = self.curriculum.quizzes.len();
        rule.weeks
            .iter()
            .all(|w| self.progress.weeks_done.contains(w))
            && rule.psets.iter().all(|w| self.pset_done_for_week(w))
            && rule
                .quiz_modules
                .iter()
                .all(|&m| self.module_quizzes_mastered(m))
            && rule.quiz_percent.is_none_or(|pct| {
                quizzes > 0 && self.quiz_correct_count() * 100 >= quizzes * pct as usize
            })
            && rule
                .challenges
                .is_none_or(|n| self.progress.challenges_solved.len() >= n as usize)
    }

    /// Выдаёт все ачивки, чьи условия выполнены; XP начисляется один раз.
    pub fn check_achievements(&mut self) {
        let earned: Vec<(String, u32, String)> = self
            .curriculum
            .achievements
            .iter()
            .filter(|a| {
                !self.progress.achievements.contains(&a.id) && self.achievement_met(&a.when)
            })
            .map(|a| (a.id.clone(), a.xp, a.name.clone()))
            .collect();
        for (id, reward, name) in earned {
            self.progress.achievements.insert(id);
            self.add_xp(reward);
            self.new_achievements.push(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(f: impl FnOnce(&mut Rule)) -> Rule {
        let mut r = Rule::default();
        f(&mut r);
        r
    }

    #[test]
    fn empty_rule_never_unlocks() {
        assert!(!AppState::in_memory().achievement_met(&Rule::default()));
    }

    #[test]
    fn week_and_pset_conditions_are_conjunctive() {
        let mut app = AppState::in_memory();
        let r = rule(|r| {
            r.weeks = vec!["w0".into()];
            r.psets = vec!["w0".into()];
        });
        assert!(!app.achievement_met(&r));
        app.toggle_week_done("w0");
        assert!(!app.achievement_met(&r), "одной недели мало");
        app.complete_pset_with_explain("w0:0", "понял".into());
        assert!(app.achievement_met(&r));
    }

    #[test]
    fn quiz_conditions_use_the_curriculum() {
        let mut app = AppState::in_memory();
        let module1 = rule(|r| r.quiz_modules = vec![1]);
        let ninety = rule(|r| r.quiz_percent = Some(90));
        assert!(!app.achievement_met(&module1) && !app.achievement_met(&ninety));
        for q in app.curriculum.quizzes.clone() {
            app.progress.quiz_correct.insert(q.id);
        }
        assert!(app.achievement_met(&module1) && app.achievement_met(&ninety));
        assert!(
            !app.achievement_met(&rule(|r| r.quiz_modules = vec![250])),
            "нет квизов — не «всё решено»"
        );
        // 90% — граница: минимально достаточное число верных проходит, на одно меньше — нет
        let total = app.curriculum.quizzes.len();
        let needed = (total * 90).div_ceil(100);
        app.progress.quiz_correct.clear();
        for q in app.curriculum.quizzes.iter().take(needed) {
            app.progress.quiz_correct.insert(q.id.clone());
        }
        assert!(app.achievement_met(&ninety));
        let last = app.progress.quiz_correct.iter().next().cloned().unwrap();
        app.progress.quiz_correct.remove(&last);
        assert!(!app.achievement_met(&ninety));
    }

    #[test]
    fn challenge_counter_rule() {
        let mut app = AppState::in_memory();
        let r = rule(|r| r.challenges = Some(2));
        app.progress.challenges_solved.insert("lv1a".into());
        assert!(!app.achievement_met(&r));
        app.progress.challenges_solved.insert("lv1b".into());
        assert!(app.achievement_met(&r));
    }

    #[test]
    fn unlocking_pays_xp_once_and_queues_a_popup() {
        let mut app = AppState::in_memory();
        let ach = app
            .curriculum
            .achievements
            .iter()
            .find(|a| a.id == "start")
            .cloned()
            .expect("ачивка start");
        app.toggle_week_done("w0");
        app.complete_pset_with_explain("w0:0", "готово".into());
        assert!(app.progress.achievements.contains("start"));
        let xp_after = app.progress.xp;
        assert!(xp_after >= ach.xp);
        app.check_achievements();
        app.check_achievements();
        assert_eq!(app.progress.xp, xp_after);
        assert_eq!(
            app.new_achievements
                .iter()
                .filter(|n| **n == ach.name)
                .count(),
            1
        );
    }
}
