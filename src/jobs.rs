//! Фоновые задачи: блокирующие операции (сеть, внешние процессы) не должны замораживать UI.

use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Состояние фоновой задачи.
#[derive(Debug, PartialEq, Eq)]
pub enum JobState<T> {
    Running,
    Done(T),
    /// Поток завершился аварийно (паника).
    Failed,
}

pub struct Job<T> {
    rx: Receiver<T>,
}

impl<T: Send + 'static> Job<T> {
    /// Запускает `work` в отдельном потоке и по готовности будит UI.
    pub fn spawn(ctx: &egui::Context, work: impl FnOnce() -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
        Job { rx }
    }

    /// Неблокирующая проверка. Результат выдаётся один раз.
    pub fn poll(&self) -> JobState<T> {
        match self.rx.try_recv() {
            Ok(v) => JobState::Done(v),
            Err(TryRecvError::Empty) => JobState::Running,
            Err(TryRecvError::Disconnected) => JobState::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait<T: Send + 'static>(job: &Job<T>) -> JobState<T> {
        let start = Instant::now();
        loop {
            match job.poll() {
                JobState::Running if start.elapsed() < Duration::from_secs(5) => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                other => return other,
            }
        }
    }

    #[test]
    fn result_arrives_without_blocking_the_caller() {
        let ctx = egui::Context::default();
        let job = Job::spawn(&ctx, || {
            std::thread::sleep(Duration::from_millis(50));
            21 * 2
        });
        assert_eq!(job.poll(), JobState::Running);
        assert_eq!(wait(&job), JobState::Done(42));
    }

    #[test]
    fn panicking_worker_is_reported_as_failed() {
        let ctx = egui::Context::default();
        let job: Job<u8> = Job::spawn(&ctx, || panic!("boom"));
        assert_eq!(wait(&job), JobState::Failed);
    }
}
