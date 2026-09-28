// RE-50 — Reverse Engineering course app (CS50-style)
// Rust + eframe/egui 0.29. Cross-platform: Linux, macOS, Windows 11.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
