// RE-50 — Reverse Engineering course app (CS50-style).
// Cross-platform: Linux, macOS, Windows 11.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use re50::crash;
use re50::state::AppState;
use re50::storage::Paths;
use std::process::ExitCode;

fn main() -> ExitCode {
    let log = crash::log_path(&Paths::detect());
    crash::install_panic_hook(log.clone());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("RE-50 — Реверс-инжиниринг с нуля"),
        ..Default::default()
    };

    let app = AppState::load_or_default();
    let result = eframe::run_native(
        "RE-50",
        options,
        Box::new(move |cc| {
            let mut app = app;
            app.configure(cc);
            Ok(Box::new(app))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let graphics = matches!(
                error,
                eframe::Error::Glutin(_)
                    | eframe::Error::NoGlutinConfigs(..)
                    | eframe::Error::OpenGL(_)
            );
            crash::report_error(&error.to_string(), graphics, &log);
            ExitCode::FAILURE
        }
    }
}
