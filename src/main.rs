// RE-50 — Reverse Engineering course app (CS50-style)
// Rust + eframe/egui 0.29. Cross-platform: Linux, macOS, Windows 11.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod challenge_blob;
mod generator_script;
mod opponent;
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
        assert!(c.drills.asm.len() + c.drills.addr.len() + c.drills.pattern.len() + c.drills.script.len() >= 69);
        assert!(!c.challenges.is_empty(), "no challenges");
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
