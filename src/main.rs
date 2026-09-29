// RE-50 — Reverse Engineering course app (CS50-style)
// Rust + eframe/egui 0.29. Cross-platform: Linux, macOS, Windows 11.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod challenge_blob;
mod generator_script;
mod opponent;
mod detector;
mod curriculum;
mod simulators;
mod state;
mod ui;

use state::AppState;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("RE-50 — Реверс-инжиниринг с нуля"),
        ..Default::default()
    };

    let app = AppState::load_or_default();
    eframe::run_native(
        "RE-50",
        options,
        Box::new(move |cc| {
            let mut app = app;
            app.configure(cc);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use crate::curriculum::Curriculum;

    #[test]
    fn curriculum_parses() {
        let c = Curriculum::load();
        assert!(c.weeks.len() >= 37, "weeks missing");
        assert!(c.quizzes.len() >= 60);
        assert!(c.drills.asm.len() + c.drills.addr.len() + c.drills.pattern.len() + c.drills.script.len() >= 89);
        assert!(c.challenges.len() >= 15, "challenges missing");
    }

    #[test]
    fn module15_has_quizzes() {
        let c = Curriculum::load();
        let m15: Vec<_> = c.quizzes.iter().filter(|q| q.module == 15).collect();
        assert!(m15.len() >= 12, "module 15 quizzes missing: {}", m15.len());
        // они попадают в re-exam пул только если student их ответил — проверяем структуру
        for q in &m15 {
            assert!(q.answers.len() == 4);
            assert!(q.correct < 4);
        }
    }

    #[test]
    fn detector_flags_theorist() {
        use crate::state::Progress;
        let c = Curriculum::load();
        let mut p = Progress::default();
        // квизы есть — практики нет
        for i in 0..15 {
            p.quiz_correct.insert(format!("q{i}"));
        }
        let findings = crate::detector::analyze(&p, &c);
        assert!(findings.iter().any(|f| f.title.contains("Теоретик")), "no theorist flag: {:?}", findings.iter().map(|f| &f.title).collect::<Vec<_>>());
        // сбалансированный прогресс — позитив
        let mut p2 = Progress::default();
        for i in 0..12 { p2.quiz_correct.insert(format!("q{i}")); }
        for i in 0..6 { p2.lab_steps_done.insert(format!("w:lab{i}")); }
        for i in 0..4 { p2.challenges_solved.insert(format!("lv{i}a")); }
        let f2 = crate::detector::analyze(&p2, &c);
        assert!(f2.iter().any(|f| f.title.contains("Баланс")), "no balance flag");
    }

    #[test]
    fn opponent_evaluates() {
        let q = &crate::opponent::QUESTIONS[0];
        // короткий ответ — 0
        assert_eq!(crate::opponent::evaluate(q, "не знаю").score, 0);
        // ответ с ключевыми словами — высокий
        let good = crate::opponent::evaluate(q,
            "lea вычисляет адрес и кладёт его в регистр, не обращаясь к памяти, а mov разыменовывает адрес и читает память; mov упадёт, если указатель невалиден");
        assert!(good.score >= 50, "score={}", good.score);
        // пустые маркеры штрафуются
        let bad = crate::opponent::evaluate(q, "это просто одно и то же, наверное, не знаю точно но думаю что просто одинаково работают всегда");
        assert!(bad.score < good.score);
        assert_eq!(crate::opponent::QUESTIONS.len(), 14);
    }

    #[test]
    fn generator_script_embedded() {
        assert!(crate::generator_script::GENERATOR_PY.contains("TEMPLATES"));
        assert!(crate::generator_script::GENERATOR_PY.len() > 3000);
    }

    #[test]
    fn challenge_binaries_embedded() {
        let c = Curriculum::load();
        for ch in &c.challenges {
            assert!(
                crate::challenge_blob::EMBEDDED_CHALLENGES.iter().any(|(n, _)| n == &format!("challenges/{}", ch.id)),
                "missing ELF blob for {}",
                ch.id
            );
            assert!(
                crate::challenge_blob::EMBEDDED_CHALLENGES.iter().any(|(n, _)| n == &format!("challenges/{}.exe", ch.id)),
                "missing EXE blob for {}",
                ch.id
            );
        }
    }

    #[test]
    fn progress_roundtrip() {
        let c2 = Curriculum::load();
        let mut st = crate::state::AppState::with_curriculum(c2);
        st.add_xp(10);
        let json = st.export_progress();
        assert!(json.contains("\"xp\":10"), "xp missing in: {}", json);
        assert!(st.import_progress(&json));
    }
}
