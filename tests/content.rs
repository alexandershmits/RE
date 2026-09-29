//! Целостность встроенного контента: ссылки, ключи ответов, покрытие курса, достижимость ачивок.

use re50::challenge_blob::EMBEDDED_CHALLENGES;
use re50::curriculum::{Curriculum, Quiz};
use re50::state::AppState;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

fn load() -> Curriculum {
    Curriculum::try_load().expect("контент разбирается")
}

fn duplicates<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    ids.filter(|id| !seen.insert(*id)).collect()
}

fn longest_index(q: &Quiz) -> Option<usize> {
    let max = q.answers.iter().map(|a| a.chars().count()).max()?;
    let winners: Vec<usize> = q
        .answers
        .iter()
        .enumerate()
        .filter(|(_, a)| a.chars().count() == max)
        .map(|(i, _)| i)
        .collect();
    (winners.len() == 1).then(|| winners[0])
}

#[test]
fn content_has_the_expected_scale() {
    let c = load();
    let drills =
        c.drills.asm.len() + c.drills.addr.len() + c.drills.pattern.len() + c.drills.script.len();
    assert!(c.weeks.len() >= 42, "weeks: {}", c.weeks.len());
    assert!(c.quizzes.len() >= 90, "quizzes: {}", c.quizzes.len());
    assert!(drills >= 89, "drills: {drills}");
    assert!(c.challenges.len() >= 15 && c.flashcards.len() >= 20 && c.placement.len() >= 20);
    assert!(c.achievements.len() >= 37 && c.resources.len() >= 35 && c.rubric.len() == 10);
}

#[test]
fn ids_are_unique() {
    let c = load();
    assert_eq!(
        duplicates(c.weeks.iter().map(|w| w.id.as_str())),
        Vec::<&str>::new()
    );
    assert_eq!(
        duplicates(c.quizzes.iter().map(|q| q.id.as_str())),
        Vec::<&str>::new()
    );
    assert_eq!(
        duplicates(c.achievements.iter().map(|a| a.id.as_str())),
        Vec::<&str>::new()
    );
    assert_eq!(
        duplicates(c.flashcards.iter().map(|f| f.id.as_str())),
        Vec::<&str>::new()
    );
    assert_eq!(
        duplicates(c.challenges.iter().map(|ch| ch.id.as_str())),
        Vec::<&str>::new()
    );
    assert_eq!(
        duplicates(c.resources.iter().map(|r| r.url.as_str())),
        Vec::<&str>::new()
    );
    let module_ids: Vec<String> = c.modules.iter().map(|m| m.id.to_string()).collect();
    assert_eq!(
        duplicates(module_ids.iter().map(String::as_str)),
        Vec::<&str>::new()
    );
}

#[test]
fn references_resolve() {
    let c = load();
    let modules: HashSet<u8> = c.modules.iter().map(|m| m.id).collect();
    for w in &c.weeks {
        assert!(
            modules.contains(&w.module),
            "неделя {}: нет модуля {}",
            w.id,
            w.module
        );
        assert!(w.num <= w.num_end, "неделя {}", w.id);
    }
    for q in &c.quizzes {
        let week = c
            .week_by_id(&q.week)
            .unwrap_or_else(|| panic!("квиз {}: нет недели {}", q.id, q.week));
        assert_eq!(
            q.module, week.module,
            "квиз {}: модуль не совпадает с модулем недели {}",
            q.id, q.week
        );
    }
    for p in &c.placement {
        assert!(
            c.week_by_id(&p.week).is_some(),
            "placement: нет недели {}",
            p.week
        );
        assert!(p.correct < p.a.len(), "placement: {}", p.q);
    }
}

#[test]
fn quizzes_are_well_formed() {
    let c = load();
    for q in &c.quizzes {
        assert_eq!(q.answers.len(), 4, "{}", q.id);
        assert!(q.correct < 4, "{}: correct={}", q.id, q.correct);
        let unique: HashSet<&String> = q.answers.iter().collect();
        assert_eq!(unique.len(), 4, "{}: повторяющиеся ответы", q.id);
        assert!(
            q.answers.iter().all(|a| !a.trim().is_empty()),
            "{}: пустой ответ",
            q.id
        );
        assert!(
            !q.question.trim().is_empty() && !q.explain.trim().is_empty(),
            "{}",
            q.id
        );
        for a in &q.answers {
            let lower = a.to_lowercase();
            assert!(
                !lower.contains("все перечисленн") && !lower.contains("ничего из перечисленн"),
                "{}: {a}",
                q.id
            );
        }
    }
}

#[test]
fn the_correct_answer_cannot_be_guessed_by_length_or_position() {
    // регресс: в 74 из 90 вопросов верный ответ был самым длинным
    let c = load();
    let n = c.quizzes.len();
    let longest = c
        .quizzes
        .iter()
        .filter(|q| longest_index(q) == Some(q.correct))
        .count();
    assert!(
        longest * 100 <= n * 35,
        "верный ответ — самый длинный в {longest} из {n} вопросов"
    );
    for q in &c.quizzes {
        let lens: Vec<usize> = q.answers.iter().map(|a| a.chars().count()).collect();
        let (min, max) = (*lens.iter().min().unwrap(), *lens.iter().max().unwrap());
        assert!(
            max as f64 <= min as f64 * 2.0,
            "{}: ответы слишком разной длины ({min}..{max})",
            q.id
        );
    }
    let mut by_position = [0usize; 4];
    for q in &c.quizzes {
        by_position[q.correct] += 1;
    }
    for (i, count) in by_position.iter().enumerate() {
        assert!(
            *count * 100 >= n * 15 && *count * 100 <= n * 35,
            "позиция {i}: {count} из {n}"
        );
    }
}

#[test]
fn every_week_and_module_has_material_and_a_quiz() {
    let c = load();
    for w in &c.weeks {
        assert!(!w.lectures.is_empty(), "неделя {} без лекций", w.id);
        assert!(!w.checkpoint.is_empty(), "неделя {} без чек-пойнта", w.id);
        assert!(
            c.quizzes.iter().any(|q| q.week == w.id),
            "неделя {} без квизов",
            w.id
        );
        assert!(
            w.case.as_deref().is_some_and(|s| !s.trim().is_empty()),
            "неделя {} без «проблемы недели»",
            w.id
        );
    }
    for m in &c.modules {
        assert!(
            c.quizzes.iter().any(|q| q.module == m.id),
            "модуль {} ({}) без квизов",
            m.id,
            m.name
        );
    }
}

#[test]
fn text_contains_nothing_the_fonts_cannot_draw() {
    let raw = [
        include_str!("../assets/curriculum.json"),
        include_str!("../assets/drills.json"),
    ]
    .concat();
    for c in raw.chars() {
        assert!(
            c != '\u{fe0f}',
            "селектор варианта U+FE0F в данных: рисуется квадратом"
        );
        assert!(
            !('\u{4e00}'..='\u{9fff}').contains(&c),
            "иероглиф {c} в русском тексте"
        );
        assert!(
            !c.is_control() || c == '\n' || c == '\r',
            "управляющий символ {:?}",
            c
        );
    }
}

#[test]
fn achievement_rules_are_valid_and_reachable() {
    let c = load();
    let modules_with_quizzes: HashSet<u8> = c.quizzes.iter().map(|q| q.module).collect();
    for a in &c.achievements {
        let r = &a.when;
        assert!(
            !r.is_empty(),
            "ачивка {} без условия — её нельзя получить",
            a.id
        );
        assert!(
            a.xp > 0 && !a.name.is_empty() && !a.desc.is_empty(),
            "{}",
            a.id
        );
        for w in &r.weeks {
            assert!(c.week_by_id(w).is_some(), "{}: нет недели {w}", a.id);
        }
        for w in &r.psets {
            let week = c
                .week_by_id(w)
                .unwrap_or_else(|| panic!("{}: нет недели {w}", a.id));
            assert!(!week.psets.is_empty(), "{}: у недели {w} нет PSet", a.id);
        }
        for m in &r.quiz_modules {
            assert!(
                modules_with_quizzes.contains(m),
                "{}: в модуле {m} нет квизов",
                a.id
            );
        }
        assert!(
            r.quiz_percent.is_none_or(|p| (1..=100).contains(&p)),
            "{}",
            a.id
        );
        assert!(
            r.challenges
                .is_none_or(|n| n as usize <= c.challenges.len()),
            "{}",
            a.id
        );
    }

    let mut app = AppState::in_memory();
    app.check_achievements();
    assert!(
        app.progress.achievements.is_empty(),
        "новичку ничего не выдаётся: {:?}",
        app.progress.achievements
    );

    for w in &c.weeks {
        app.progress.weeks_done.insert(w.id.clone());
        for i in 0..w.psets.len() {
            app.progress.psets_done.insert(format!("{}:{i}", w.id));
        }
    }
    app.progress
        .quiz_correct
        .extend(c.quizzes.iter().map(|q| q.id.clone()));
    app.progress
        .challenges_solved
        .extend(c.challenges.iter().map(|ch| ch.id.clone()));
    app.check_achievements();
    let missing: Vec<&str> = c
        .achievements
        .iter()
        .filter(|a| !app.progress.achievements.contains(&a.id))
        .map(|a| a.id.as_str())
        .collect();
    assert!(missing.is_empty(), "недостижимые ачивки: {missing:?}");
}

#[test]
fn weeks_without_psets_or_only_week_progress_do_not_unlock_pset_achievements() {
    let c = load();
    let mut app = AppState::in_memory();
    for w in &c.weeks {
        app.progress.weeks_done.insert(w.id.clone());
    }
    app.check_achievements();
    let unlocked: BTreeSet<&str> = app
        .progress
        .achievements
        .iter()
        .map(String::as_str)
        .collect();
    for a in &c.achievements {
        if !a.when.psets.is_empty() {
            assert!(
                !unlocked.contains(a.id.as_str()),
                "{} выдана без сданных PSet",
                a.id
            );
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn challenge_binaries_match_the_curriculum() {
    let c = load();
    let embedded: BTreeMap<&str, &[u8]> =
        EMBEDDED_CHALLENGES.iter().map(|(n, b)| (*n, *b)).collect();
    for ch in &c.challenges {
        let elf = embedded
            .get(format!("challenges/{}", ch.id).as_str())
            .unwrap_or_else(|| panic!("нет ELF {}", ch.id));
        let exe = embedded
            .get(format!("challenges/{}.exe", ch.id).as_str())
            .unwrap_or_else(|| panic!("нет EXE {}", ch.id));
        assert_eq!(&elf[..4], b"\x7fELF", "{}", ch.id);
        assert_eq!(&exe[..2], b"MZ", "{}", ch.id);
        assert!(
            hex(&Sha256::digest(elf)).starts_with(&ch.sha256),
            "{}: SHA-256 ELF не совпадает с curriculum.json",
            ch.id
        );
        assert!(
            hex(&Sha256::digest(exe)).starts_with(&ch.sha256_exe),
            "{}: SHA-256 EXE не совпадает",
            ch.id
        );
        let flag = format!("FLAG{{{}}}", ch.flag);
        for (name, bytes) in [("ELF", elf), ("EXE", exe)] {
            assert!(
                bytes.windows(flag.len()).any(|w| w == flag.as_bytes()),
                "{}: флаг {flag} не найден в {name}",
                ch.id
            );
        }
        assert!((1..=5).contains(&ch.level), "{}", ch.id);
    }
}

#[test]
fn embedded_list_equals_the_assets_directory() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/challenges");
    let mut on_disk = BTreeSet::new();
    let mut sources = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        match name.rsplit_once('.') {
            Some((stem, "c")) => {
                sources.insert(stem.to_string());
            }
            _ => {
                on_disk.insert(format!("challenges/{name}"));
            }
        }
    }
    let embedded: BTreeSet<String> = EMBEDDED_CHALLENGES
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    assert_eq!(
        embedded, on_disk,
        "список в challenge_blob.rs разошёлся с assets/challenges/"
    );
    let c = load();
    for ch in &c.challenges {
        assert!(
            sources.contains(&ch.id),
            "нет исходника {}.c — бинарь не воспроизвести",
            ch.id
        );
    }
}

fn hex_numbers(s: &str) -> Vec<u128> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|t| t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")))
        .filter_map(|t| u128::from_str_radix(t, 16).ok())
        .collect()
}

#[test]
fn address_drills_have_correct_answer_keys() {
    let c = load();
    for (i, d) in c.drills.addr.iter().enumerate() {
        let n = hex_numbers(&d.q);
        let expected = if d.q.contains("VA точки входа") {
            n[0] + n[1] // ImageBase + RVA
        } else if d.q.contains("Её RVA") {
            n[1] - n[0] // VA − ImageBase
        } else if d.q.contains("лежит на RVA") {
            n[0] + n[2] - n[1] // RVA секции + (offset − PointerToRawData)
        } else if d.q.contains("Чему равен ImageBase") {
            n[0] - n[1] // VA − RVA
        } else {
            panic!("addr{i}: неизвестный тип задачи: {}", d.q)
        };
        let key = hex_numbers(&d.answers[d.correct]);
        assert_eq!(
            key,
            vec![expected],
            "addr{i}: {} → отмечено {}, верно {expected:#X}",
            d.q,
            d.answers[d.correct]
        );
    }
}

fn check_choices<'a>(name: &str, items: impl Iterator<Item = (&'a Vec<String>, usize)>) {
    for (i, (answers, correct)) in items.enumerate() {
        let unique: HashSet<&String> = answers.iter().collect();
        assert!(
            answers.len() == 4 && unique.len() == 4 && correct < 4,
            "{name}{i}"
        );
    }
}

#[test]
fn drills_are_well_formed() {
    let c = load();
    check_choices("asm", c.drills.asm.iter().map(|d| (&d.answers, d.correct)));
    check_choices(
        "addr",
        c.drills.addr.iter().map(|d| (&d.answers, d.correct)),
    );
    check_choices(
        "pattern",
        c.drills.pattern.iter().map(|d| (&d.answers, d.correct)),
    );
    for (i, d) in c.drills.script.iter().enumerate() {
        assert!(
            !d.task.trim().is_empty() && !d.answer.trim().is_empty() && !d.hint.trim().is_empty(),
            "scr{i}"
        );
    }
}

#[test]
fn resources_are_secure_links() {
    let c = load();
    for r in &c.resources {
        assert!(r.url.starts_with("https://"), "{}: {}", r.name, r.url);
        assert!(
            !r.name.trim().is_empty() && !r.category.trim().is_empty(),
            "{}",
            r.url
        );
    }
}

/// Поставляемые Linux-бинари ведут себя так, как обещает curriculum.json: верный ввод даёт флаг, мусор — нет.
#[cfg(target_os = "linux")]
#[test]
fn shipped_linux_binaries_accept_the_solution_and_reject_garbage() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    let solutions: BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("../tools/challenge_solutions.json"))
            .expect("решения разбираются");
    let dir = std::env::temp_dir().join(format!("re50-challenges-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let run = |path: &Path, lines: &[String]| -> String {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("запуск бинаря");
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(format!("{}\n", lines.join("\n")).as_bytes())
            .unwrap();
        drop(stdin);
        String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
    };
    let c = load();
    for ch in &c.challenges {
        let bytes = EMBEDDED_CHALLENGES
            .iter()
            .find(|(n, _)| *n == format!("challenges/{}", ch.id))
            .unwrap()
            .1;
        let path = dir.join(&ch.id);
        std::fs::write(&path, bytes).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let flag = format!("FLAG{{{}}}", ch.flag);
        let solution = solutions
            .get(&ch.id)
            .unwrap_or_else(|| panic!("нет решения для {}", ch.id));
        assert!(
            run(&path, solution).contains(&flag),
            "{}: верный ввод не даёт флаг",
            ch.id
        );
        let garbage = vec!["zzzzzz".to_string(); 3];
        assert!(
            !run(&path, &garbage).contains(&flag),
            "{}: мусорный ввод принят",
            ch.id
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
