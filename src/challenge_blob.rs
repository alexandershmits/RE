//! Челлендж-бинари, вшитые в приложение (ELF для Linux, PE для Windows).
//! Список обязан совпадать с `assets/challenges/` — это проверяет тест.

macro_rules! embed {
    ($($name:literal),+ $(,)?) => {
        &[$((
            concat!("challenges/", $name),
            include_bytes!(concat!("../assets/challenges/", $name)),
        )),+]
    };
}

pub const EMBEDDED_CHALLENGES: &[(&str, &[u8])] = embed![
    "lv1a", "lv1a.exe", "lv1b", "lv1b.exe", "lv1c", "lv1c.exe", "lv2a", "lv2a.exe", "lv2b",
    "lv2b.exe", "lv2c", "lv2c.exe", "lv2d", "lv2d.exe", "lv3a", "lv3a.exe", "lv3b", "lv3b.exe",
    "lv3c", "lv3c.exe", "lv4a", "lv4a.exe", "lv4b", "lv4b.exe", "lv4c", "lv4c.exe", "lv5a",
    "lv5a.exe", "lv5b", "lv5b.exe",
];
