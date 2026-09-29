//! Документация не расходится с кодом: цифры в README, версия в CHANGELOG, лицензии, подзаголовок курса.

use re50::curriculum::Curriculum;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn read(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Внешние ссылки в данных курса — теми же правилами, что и `tools/check_links.py`.
fn count_urls(value: &serde_json::Value, found: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(s) => {
            for prefix in ["https://", "http://"] {
                for (start, _) in s.match_indices(prefix) {
                    let url: String = s[start..]
                        .chars()
                        .take_while(|c| !c.is_whitespace() && !")]»\"'<>".contains(*c))
                        .collect();
                    found.insert(url.trim_end_matches(['.', ',', ';', ':']).to_string());
                }
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| count_urls(v, found)),
        serde_json::Value::Object(map) => map.values().for_each(|v| count_urls(v, found)),
        _ => {}
    }
}

fn last_week(c: &Curriculum) -> usize {
    c.weeks.iter().map(|w| w.num_end as usize).max().unwrap()
}

#[test]
fn readme_statistics_match_the_content() {
    let readme = read("README.md");
    let marker = readme
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("<!-- stats:")
                .and_then(|r| r.strip_suffix("-->"))
        })
        .expect("в README нет маркера `<!-- stats: … -->`");
    let stated: BTreeMap<String, usize> = marker
        .split_whitespace()
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.parse().expect("число")))
        .collect();
    let c = Curriculum::load();
    let d = &c.drills;
    let mut urls = BTreeSet::new();
    let data: serde_json::Value = serde_json::from_str(&read("assets/curriculum.json")).unwrap();
    count_urls(&data, &mut urls);
    let actual: BTreeMap<String, usize> = [
        ("modules", c.modules.len()),
        ("week_entries", c.weeks.len()),
        ("last_week", last_week(&c)),
        ("psets", c.weeks.iter().map(|w| w.psets.len()).sum()),
        ("quizzes", c.quizzes.len()),
        (
            "drills",
            d.asm.len() + d.addr.len() + d.pattern.len() + d.script.len(),
        ),
        ("flashcards", c.flashcards.len()),
        ("achievements", c.achievements.len()),
        ("challenges", c.challenges.len()),
        ("resources", c.resources.len()),
        ("urls", urls.len()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    assert_eq!(
        stated, actual,
        "цифры в README устарели — обновите маркер и текст"
    );
    for (key, text) in [
        ("modules", "17 модулей"),
        ("last_week", "48 недель"),
        ("week_entries", "(42 страницы)"),
        ("psets", "41 problem set"),
        ("quizzes", "108 квизов"),
        ("drills", "Дриллы (89)"),
        ("achievements", "37 ачивок"),
        ("challenges", "15 crackme"),
        ("resources", "35 ресурсов"),
        ("urls", "39 внешних ссылок"),
    ] {
        let n: String = text.chars().filter(char::is_ascii_digit).collect();
        assert_eq!(n, actual[key].to_string(), "{key}: {text}");
        assert!(
            readme.contains(text) || key == "drills",
            "в README нет фразы «{text}»"
        );
    }
}

#[test]
fn course_subtitle_matches_the_number_of_weeks() {
    let c = Curriculum::load();
    let subtitle = &c.course.subtitle;
    assert!(
        subtitle.starts_with(&format!("{} недель", last_week(&c))),
        "подзаголовок: {subtitle}"
    );
}

#[test]
fn changelog_starts_with_the_package_version() {
    let changelog = read("CHANGELOG.md");
    let first = changelog
        .lines()
        .find(|l| l.starts_with("## ["))
        .expect("в CHANGELOG нет записей");
    assert!(
        first.starts_with(&format!("## [{}]", env!("CARGO_PKG_VERSION"))),
        "{first}"
    );
}

#[test]
fn licenses_and_notices_are_present() {
    assert!(read("LICENSE").starts_with("MIT License"));
    assert!(read("assets/fonts/OFL.txt").contains("SIL OPEN FONT LICENSE"));
    let notices = read("THIRD_PARTY_NOTICES.md");
    assert!(notices.contains("Noto Emoji") && notices.contains("OFL.txt"));
    assert!(read("Cargo.toml").contains("license = \"MIT\""));
}

#[test]
fn images_referenced_by_readme_exist() {
    let readme = read("README.md");
    let mut found = 0;
    for part in readme.split("src=\"").skip(1) {
        let path = part.split('"').next().unwrap();
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(path).is_file(),
            "нет файла {path}"
        );
        found += 1;
    }
    assert!(found >= 4, "в README должны быть скриншоты");
}
