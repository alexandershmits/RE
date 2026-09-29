//! RE-50 — приложение курса по реверс-инжинирингу (Rust + egui).

pub mod challenge_blob;
pub mod curriculum;
pub mod detector;
pub mod emulator;
pub mod generator_script;
pub mod jobs;
pub mod opponent;
pub mod rng;
pub mod simulators;
pub mod state;
pub mod storage;
pub mod ui;
pub mod util;

#[cfg(test)]
pub(crate) mod testutil;
