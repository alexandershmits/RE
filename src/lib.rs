//! RE-50 — приложение курса по реверс-инжинирингу (Rust + egui).

pub mod challenge_blob;
pub mod curriculum;
pub mod detector;
pub mod generator_script;
pub mod opponent;
pub mod simulators;
pub mod state;
pub mod ui;

#[cfg(test)]
mod tests {
    use crate::curriculum::Curriculum;

    #[test]
    fn curriculum_parses() {
        let c = Curriculum::load();
        assert!(c.weeks.len() >= 42, "weeks missing");
        assert!(c.quizzes.len() >= 90);
        assert!(
            c.drills.asm.len()
                + c.drills.addr.len()
                + c.drills.pattern.len()
                + c.drills.script.len()
                >= 89
        );
        assert!(c.challenges.len() >= 15, "challenges missing");
    }

    #[test]
    fn search_finds_content() {
        let app = crate::state::AppState::for_test();
        let r = app.search_course("Ghidra");
        assert!(!r.is_empty(), "search for Ghidra found nothing");
        let r2 = app.search_course("x");
        assert!(r2.is_empty(), "too-short query must return empty");
        let r3 = app.search_course("несуществующееслово123");
        assert!(r3.is_empty());
    }

    #[test]
    fn week_lab_export() {
        let mut app = crate::state::AppState::for_test();
        let dir = app.export_week_lab("w44").expect("export failed");
        let task = std::path::PathBuf::from(&dir).join("TASK.md");
        let content = std::fs::read_to_string(&task).expect("TASK.md missing");
        assert!(
            content.contains("1-day") || content.contains("патч") || content.contains("PSet38")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn profile_export_import_roundtrip() {
        let mut app = crate::state::AppState::for_test();
        app.progress.xp = 777;
        app.progress.theme = "light".into();
        let p = app.export_profile_file().expect("export failed");
        app.progress.xp = 0;
        app.progress.theme = "dark".into();
        app.import_profile_file().expect("import failed");
        assert_eq!(app.progress.xp, 777);
        assert_eq!(app.progress.theme, "light");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn journal_export_writes_file() {
        let mut app = crate::state::AppState::for_test();
        app.progress.journal = "Проверка экспорта".into();
        let p = app.export_journal().expect("export failed");
        let content = std::fs::read_to_string(&p).expect("file missing");
        assert!(content.contains("Проверка экспорта"));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn work_session_methodology() {
        use crate::state::WorkSession;
        let mut w = WorkSession::new();
        // порядок ок: триаж -> статика -> динамика
        w.stages_done = vec!["triage".into(), "static".into(), "dynamic".into()];
        assert!(w.methodology_ok().is_ok());
        // нарушение: динамика до триажа
        w.stages_done = vec!["dynamic".into(), "triage".into()];
        assert!(w.methodology_ok().is_err());
        // отчёт: полнота
        let mut w2 = WorkSession::new();
        assert_eq!(w2.report_completeness(), 0);
        w2.report[0] = "ELF64, gcc".into();
        w2.report[1] = "sha256: abcd".into();
        w2.report[2] = "без упаковки".into();
        assert_eq!(w2.report_completeness(), 30);
    }

    #[test]
    fn generative_simulators() {
        use crate::simulators::{check_gen, generate, GenKind};
        // детерминизм: тот же seed — та же задача
        let t1 = generate(&GenKind::RipRelative, 42);
        let t2 = generate(&GenKind::RipRelative, 42);
        assert_eq!(t1.question, t2.question);
        assert!(check_gen(&t1, &t2.answer));
        let t3 = generate(&GenKind::RipRelative, 43);
        let _ = t3;
        // check_gen принимает 0x префикс
        assert!(check_gen(&t1, &format!("0x{}", t1.answer)));
        // LE и decode работают
        let le = generate(&GenKind::LittleEndian, 7);
        assert!(check_gen(&le, &le.answer));
        let dec = generate(&GenKind::DecodeMov, 9);
        assert!(check_gen(&dec, &dec.answer));
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
        assert!(
            findings.iter().any(|f| f.title.contains("Теоретик")),
            "no theorist flag: {:?}",
            findings.iter().map(|f| &f.title).collect::<Vec<_>>()
        );
        // сбалансированный прогресс — позитив
        let mut p2 = Progress::default();
        for i in 0..12 {
            p2.quiz_correct.insert(format!("q{i}"));
        }
        for i in 0..6 {
            p2.lab_steps_done.insert(format!("w:lab{i}"));
        }
        for i in 0..4 {
            p2.challenges_solved.insert(format!("lv{i}a"));
        }
        let f2 = crate::detector::analyze(&p2, &c);
        assert!(
            f2.iter().any(|f| f.title.contains("Баланс")),
            "no balance flag"
        );
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
                crate::challenge_blob::EMBEDDED_CHALLENGES
                    .iter()
                    .any(|(n, _)| n == &format!("challenges/{}", ch.id)),
                "missing ELF blob for {}",
                ch.id
            );
            assert!(
                crate::challenge_blob::EMBEDDED_CHALLENGES
                    .iter()
                    .any(|(n, _)| n == &format!("challenges/{}.exe", ch.id)),
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
