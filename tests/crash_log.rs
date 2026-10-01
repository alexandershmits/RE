//! Хук паники пишет журнал для любого потока. Хук глобальный, поэтому всё в одном тесте этого процесса.

use re50::crash;

#[test]
fn panics_are_logged_with_thread_message_and_location() {
    let dir = std::env::temp_dir().join(format!("re50-crash-test-{}", std::process::id()));
    let log = dir.join("crash.log");
    let _ = std::fs::remove_dir_all(&dir);
    crash::install_panic_hook(log.clone());

    let worker = std::thread::Builder::new()
        .name("фоновая-задача".into())
        .spawn(|| panic!("сломалось в задаче"))
        .unwrap();
    assert!(worker.join().is_err(), "паника потока должна дойти до join");

    let text = std::fs::read_to_string(&log).expect("журнал создан хуком");
    // на Windows путь к файлу пишется с обратной косой чертой
    let normalized = text.replace('\\', "/");
    for needle in [
        "паника в потоке «фоновая-задача»",
        "сломалось в задаче",
        "tests/crash_log.rs",
    ] {
        assert!(normalized.contains(needle), "{needle}:\n{text}");
    }
    assert!(
        text.starts_with("=== "),
        "запись начинается с заголовка: {text}"
    );

    let second = std::thread::spawn(|| panic!("вторая паника"));
    let _ = second.join();
    let text = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        text.matches("=== ").count(),
        2,
        "записи дописываются, а не затирают друг друга:\n{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
