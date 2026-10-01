//! Журнал сбоев. В релизной сборке под Windows у программы нет консоли (`windows_subsystem = "windows"`):
//! паника или ошибка запуска (например, нет OpenGL 2.0) иначе пропали бы бесследно — окно просто не появилось бы.

use crate::storage::Paths;
use crate::util;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const LOG_NAME: &str = "crash.log";
/// Больше этого журнал сдвигается в `crash.log.old`, чтобы не расти бесконечно.
const MAX_LOG_BYTES: u64 = 256 * 1024;
/// Сколько символов трассировки попадает в запись журнала и сколько текста ошибки — в окно сообщения
/// (в журнале остаётся полный текст).
const MAX_TRACE_CHARS: usize = 8_000;
const MAX_DIALOG_CHARS: usize = 300;

/// Журнал лежит рядом с прогрессом; без каталога настроек — во временной папке.
pub fn log_path(paths: &Paths) -> PathBuf {
    paths.config_dir.as_ref().map_or_else(
        || std::env::temp_dir().join("re50-crash.log"),
        |dir| dir.join(LOG_NAME),
    )
}

/// Дописывает запись в журнал; старый (больше `MAX_LOG_BYTES`) уходит в `.old`.
pub fn append(path: &Path, entry: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_LOG_BYTES) {
        let name = path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let _ = std::fs::rename(path, path.with_file_name(format!("{name}.old")));
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())
}

fn header(now: u64) -> String {
    format!(
        "=== {} UTC · RE-50 {} · {}-{} ===\n",
        util::format_datetime(now),
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn truncated(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// Запись о панике: поток, текст, место и трассировка.
pub fn panic_entry(
    now: u64,
    thread: &str,
    message: &str,
    location: Option<&str>,
    trace: &str,
) -> String {
    let mut entry = header(now);
    entry.push_str(&format!("паника в потоке «{thread}»: {message}\n"));
    if let Some(location) = location {
        entry.push_str(&format!("в {location}\n"));
    }
    entry.push_str(&truncated(trace, MAX_TRACE_CHARS));
    entry.push_str("\n\n");
    entry
}

/// Запись об ошибке, с которой приложение не смогло открыть окно или завершилось.
pub fn error_entry(now: u64, error: &str) -> String {
    format!(
        "{}ошибка: {}\n\n",
        header(now),
        truncated(error, MAX_TRACE_CHARS)
    )
}

/// Текст для пользователя; `opengl` — ошибка связана с графикой (главная причина молчаливого отказа запуска).
pub fn error_message(error: &str, opengl: bool, log: &Path) -> String {
    let mut text = format!(
        "RE-50 завершилась с ошибкой.\n\n{}\n",
        truncated(error, MAX_DIALOG_CHARS)
    );
    if opengl {
        text.push_str(
            "\nПохоже, не работает графика: нужен OpenGL 2.0 или новее. Обновите драйвер видеокарты; \
             в виртуальной машине или удалённом сеансе включите 3D-ускорение.\n",
        );
    }
    text.push_str(&format!("\nПодробности записаны в {}", log.display()));
    text
}

/// Записывает ошибку в журнал и показывает её: окном на Windows (там нет консоли), в stderr — везде.
pub fn report_error(error: &str, opengl: bool, log: &Path) {
    let _ = append(log, &error_entry(util::unix_now(), error));
    let text = error_message(error, opengl, log);
    eprintln!("{text}");
    message_box("RE-50", &text);
}

/// Хук паники: запись в журнал для любого потока; окно — только если упал главный поток (паника фонового
/// потока ловится `Job` и показывается как ошибка задачи, а не как сбой приложения).
pub fn install_panic_hook(log: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("без имени");
        let message = info.payload_as_str().unwrap_or("не текст");
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let trace = std::backtrace::Backtrace::force_capture().to_string();
        let entry = panic_entry(util::unix_now(), name, message, location.as_deref(), &trace);
        let _ = append(&log, &entry);
        previous(info);
        if name == "main" {
            let text = format!(
                "RE-50 неожиданно завершилась.\n\n{}\n\nПодробности записаны в {}",
                truncated(message, MAX_DIALOG_CHARS),
                log.display()
            );
            message_box("RE-50", &text);
        }
    }));
}

#[cfg(windows)]
fn message_box(title: &str, text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let (title, text) = (wide(title), wide(text));
    // SAFETY: обе строки заканчиваются нулём и живут до возврата из вызова; окна-владельца нет.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn message_box(_title: &str, _text: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn log_lives_next_to_the_progress_file() {
        let dir = TempDir::new("crashpath");
        assert_eq!(
            log_path(&Paths::in_dir(dir.path())),
            dir.path().join("config").join(LOG_NAME)
        );
        assert!(log_path(&Paths::none()).starts_with(std::env::temp_dir()));
    }

    #[test]
    fn entries_say_when_where_and_what() {
        let entry = panic_entry(
            1_000_000_000,
            "main",
            "индекс за границей",
            Some("src/ui/mod.rs:10:5"),
            "кадр 0",
        );
        assert!(
            entry.starts_with("=== 2001-09-09 01:46:40 UTC · RE-50 "),
            "{entry}"
        );
        for needle in [
            "паника в потоке «main»",
            "индекс за границей",
            "в src/ui/mod.rs:10:5",
            "кадр 0",
        ] {
            assert!(entry.contains(needle), "{needle}: {entry}");
        }
        assert!(error_entry(0, "нет OpenGL").contains("ошибка: нет OpenGL"));
    }

    #[test]
    fn long_traces_are_cut() {
        let entry = panic_entry(0, "t", "m", None, &"x".repeat(50_000));
        assert!(entry.chars().count() < MAX_TRACE_CHARS + 300);
        assert!(entry.contains('…'));
    }

    #[test]
    fn log_appends_and_rotates_when_it_grows() {
        let dir = TempDir::new("crashlog");
        let log = dir.path().join("nested").join(LOG_NAME);
        append(&log, "первая\n").unwrap();
        append(&log, "вторая\n").unwrap();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "первая\nвторая\n");
        std::fs::write(&log, "ы".repeat(MAX_LOG_BYTES as usize)).unwrap();
        append(&log, "новая\n").unwrap();
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "новая\n");
        assert!(
            std::fs::metadata(log.with_file_name("crash.log.old"))
                .unwrap()
                .len()
                > MAX_LOG_BYTES
        );
    }

    #[test]
    fn the_message_names_the_log_and_explains_opengl() {
        let log = Path::new("/home/a/.config/re50/crash.log");
        let plain = error_message("winit error: boom", false, log);
        assert!(
            plain.contains("winit error: boom") && plain.contains("/home/a/.config/re50/crash.log")
        );
        assert!(!plain.contains("OpenGL"));
        let gl = error_message("glutin error: no config", true, log);
        assert!(gl.contains("OpenGL 2.0") && gl.contains("драйвер"));
        assert!(
            error_message(&"э".repeat(5_000), false, log)
                .chars()
                .count()
                < 600
        );
    }
}
