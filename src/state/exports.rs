//! Экспорт и импорт: профиль, журнал, лабы недель, бинари челленджей.

use super::{AppState, Progress};
use crate::challenge_blob::EMBEDDED_CHALLENGES;
use crate::util;
use std::path::Path;

fn embedded(name: &str) -> Option<&'static [u8]> {
    EMBEDDED_CHALLENGES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, bytes)| *bytes)
}

fn write(path: &Path, data: impl AsRef<[u8]>) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

fn make_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("{}: {e}", path.display()))
}

impl AppState {
    /// Корень экспортов; создаётся при необходимости.
    fn export_root(&self) -> Result<std::path::PathBuf, String> {
        let root = self.paths.export_root()?.to_path_buf();
        make_dir(&root)?;
        Ok(root)
    }

    /// `<экспорт>/re50-lab/week_<id>/TASK.md`: лекции, шаги, PSet, чек-пойнт.
    pub fn export_week_lab(&self, week_id: &str) -> Result<String, String> {
        let w = self
            .curriculum
            .week_by_id(week_id)
            .ok_or_else(|| format!("нет недели {week_id}"))?;
        let dir = self
            .export_root()?
            .join("re50-lab")
            .join(format!("week_{}", w.id));
        make_dir(&dir)?;
        let mut md = format!("# {} — {}\n\n## Лекции\n", w.label(), w.title);
        for l in &w.lectures {
            md.push_str(&format!("- {l}\n"));
        }
        if let Some(lab) = &w.lab {
            md.push_str(&format!("\n## {}\n", lab.title));
            for (i, s) in lab.steps.iter().enumerate() {
                md.push_str(&format!("{}. {s}\n", i + 1));
            }
        }
        md.push_str("\n## Problem Set\n");
        for p in &w.psets {
            md.push_str(&format!("- {p}\n"));
        }
        md.push_str("\n## Чекпоинт (самопроверка)\n");
        for c in &w.checkpoint {
            md.push_str(&format!("- [ ] {c}\n"));
        }
        if let Some(case) = &w.case {
            md.push_str(&format!("\n## Проблема недели\n{case}\n"));
        }
        write(&dir.join("TASK.md"), md)?;
        Ok(dir.display().to_string())
    }

    /// Бинари челленджа (ELF + PE) и TASK.txt в `<экспорт>/re50-lab/<id>/`.
    pub fn export_challenge(&self, id: &str) -> Result<String, String> {
        let ch = self
            .curriculum
            .challenges
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| format!("нет челленджа {id}"))?;
        let dir = self.export_root()?.join("re50-lab").join(id);
        make_dir(&dir)?;
        let elf = embedded(&format!("challenges/{id}")).ok_or("в сборке нет ELF-бинаря")?;
        let exe = embedded(&format!("challenges/{id}.exe")).ok_or("в сборке нет PE-бинаря")?;
        let elf_path = dir.join(id);
        write(&elf_path, elf)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&elf_path, std::fs::Permissions::from_mode(0o755));
        }
        write(&dir.join(format!("{id}.exe")), exe)?;
        let note = format!(
            "Челлендж: {} (уровень {})\nЗадача: {}\nПодсказка: {}\n\n\
             SHA-256 (первые 16 символов): {id} = {}, {id}.exe = {}\n\n\
             Запуск:\n  Linux: ./{id}\n  Windows: {id}.exe\n\
             Запускайте только в виртуальной машине со снапшотом.\n\
             Решите в Ghidra/x64dbg и введите флаг в приложении.\n",
            ch.title, ch.level, ch.desc, ch.hint, ch.sha256, ch.sha256_exe
        );
        write(&dir.join("TASK.txt"), note)?;
        Ok(dir.display().to_string())
    }

    /// Прогресс одной строкой JSON (копирование в буфер).
    pub fn export_progress(&self) -> String {
        serde_json::to_string(&self.progress).unwrap_or_default()
    }

    /// Заменяет прогресс импортированным. Перед заменой текущий сохраняется в резервную копию.
    pub fn import_progress(&mut self, json: &str) -> Result<(), String> {
        let mut imported: Progress =
            serde_json::from_str(json).map_err(|e| format!("некорректный JSON: {e}"))?;
        imported.sanitize();
        self.flush(true);
        self.storage.rotate_backups();
        imported.last_tick = self.progress.last_tick;
        self.progress = imported;
        self.mark_dirty();
        self.check_achievements();
        Ok(())
    }

    /// Профиль для переноса на другую машину: `<экспорт>/re50-profile.json`.
    pub fn export_profile_file(&self) -> Result<String, String> {
        let path = self.export_root()?.join("re50-profile.json");
        let json = serde_json::to_string_pretty(&self.progress).map_err(|e| e.to_string())?;
        write(&path, json)?;
        Ok(path.display().to_string())
    }

    pub fn import_profile_file(&mut self) -> Result<String, String> {
        let path = self.export_root()?.join("re50-profile.json");
        let data = std::fs::read_to_string(&path)
            .map_err(|e| format!("нет файла {}: {e}", path.display()))?;
        self.import_progress(&data)?;
        Ok(format!("Профиль импортирован из {}", path.display()))
    }

    /// Журнал, ставки и тикеты в `<экспорт>/re50-journal.md`.
    pub fn export_journal(&self) -> Result<String, String> {
        let path = self.export_root()?.join("re50-journal.md");
        let p = &self.progress;
        let mut md = String::from("# RE-50 — Журнал обучения\n\n");
        md.push_str(&format!(
            "Экспортирован: {}\n\n",
            util::format_day(util::unix_day(util::unix_now()))
        ));
        md.push_str(&format!(
            "XP: {} | Недель закрыто: {}\n\n## Журнал\n\n",
            p.xp,
            self.weeks_done_count()
        ));
        md.push_str(&p.journal);
        md.push_str("\n\n## Гипотезы (ставки)\n\n");
        for (id, bet) in &p.challenge_bets {
            let mark = match p.bet_results.get(id) {
                Some(true) => "✔",
                Some(false) => "✖",
                None => "⏳",
            };
            md.push_str(&format!("- {mark} **{id}**: {bet}\n"));
        }
        md.push_str("\n## Тикеты (рабочие сессии)\n\n");
        for r in &p.work_history {
            let mark = if r.method_ok { "✔" } else { "⚠" };
            md.push_str(&format!(
                "- {mark} {} — {} сек, отчёт {}%\n",
                r.challenge, r.seconds, r.completeness
            ));
        }
        write(&path, md)?;
        Ok(path.display().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Paths;
    use crate::testutil::TempDir;

    fn app(dir: &TempDir) -> AppState {
        AppState::with_paths(Paths::in_dir(dir.path()))
    }

    #[test]
    fn week_lab_is_written_under_the_export_dir() {
        let dir = TempDir::new("lab");
        let out = app(&dir).export_week_lab("w44").expect("экспорт");
        assert!(out.starts_with(&dir.path().display().to_string()));
        let task = std::fs::read_to_string(Path::new(&out).join("TASK.md")).unwrap();
        assert!(
            task.contains("## Лекции")
                && task.contains("## Problem Set")
                && task.contains("PSet38"),
            "{task}"
        );
        assert!(app(&dir).export_week_lab("нет-такой").is_err());
    }

    #[test]
    fn challenge_export_ships_both_binaries_and_a_note() {
        let dir = TempDir::new("challenge");
        let out = app(&dir).export_challenge("lv1a").expect("экспорт");
        let out = Path::new(&out);
        assert_eq!(&std::fs::read(out.join("lv1a")).unwrap()[..4], b"\x7fELF");
        assert_eq!(&std::fs::read(out.join("lv1a.exe")).unwrap()[..2], b"MZ");
        let note = std::fs::read_to_string(out.join("TASK.txt")).unwrap();
        assert!(note.contains("SHA-256") && note.contains("./lv1a"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(out.join("lv1a"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0o111
            );
        }
        assert!(app(&dir).export_challenge("lv9z").is_err());
    }

    #[test]
    fn profile_roundtrip_through_the_file() {
        let dir = TempDir::new("profile");
        let mut a = app(&dir);
        a.progress.xp = 777;
        a.progress.theme = "light".into();
        a.export_profile_file().unwrap();
        a.progress.xp = 0;
        a.progress.theme = "dark".into();
        a.import_profile_file().unwrap();
        assert_eq!((a.progress.xp, a.progress.theme.as_str()), (777, "light"));
    }

    #[test]
    fn import_keeps_a_backup_of_the_replaced_progress() {
        let dir = TempDir::new("importbackup");
        let mut a = app(&dir);
        a.progress.xp = 500;
        a.mark_dirty();
        a.import_progress(r#"{"xp": 1}"#).unwrap();
        assert_eq!(a.progress.xp, 1);
        let bak = std::fs::read_to_string(dir.path().join("config/progress.json.bak1")).unwrap();
        assert!(
            bak.contains("\"xp\": 500"),
            "прежний прогресс должен остаться в bak1"
        );
    }

    #[test]
    fn broken_import_changes_nothing() {
        let mut a = AppState::in_memory();
        a.progress.xp = 9;
        assert!(a
            .import_progress("не json")
            .unwrap_err()
            .contains("некорректный"));
        assert_eq!(a.progress.xp, 9);
    }

    #[test]
    fn imported_settings_are_sanitized() {
        let mut a = AppState::in_memory();
        a.import_progress(r#"{"theme": "???", "font_scale": 50.0}"#)
            .unwrap();
        assert_eq!(
            (a.progress.theme.as_str(), a.progress.font_scale),
            ("dark", 2.0)
        );
    }

    #[test]
    fn journal_export_contains_journal_bets_and_tickets() {
        let dir = TempDir::new("journal");
        let mut a = app(&dir);
        a.progress.journal = "Проверка экспорта".into();
        a.progress
            .challenge_bets
            .insert("lv1a".into(), "strcmp".into());
        a.progress.bet_results.insert("lv1a".into(), true);
        a.progress.work_history.push(crate::state::WorkRecord {
            challenge: "lv2a".into(),
            seconds: 90,
            method_ok: false,
            completeness: 40,
        });
        let md = std::fs::read_to_string(a.export_journal().unwrap()).unwrap();
        assert!(md.contains("Проверка экспорта") && md.contains("✔ **lv1a**: strcmp"));
        assert!(md.contains("⚠ lv2a — 90 сек, отчёт 40%"));
    }

    #[test]
    fn export_without_a_home_directory_is_an_error_not_the_cwd() {
        // регресс: раньше без HOME файлы молча уходили в текущий каталог
        let a = AppState::in_memory();
        for r in [
            a.export_journal(),
            a.export_profile_file(),
            a.export_week_lab("w0"),
            a.export_challenge("lv1a"),
        ] {
            assert!(r.unwrap_err().contains("RE50_EXPORT_DIR"));
        }
    }
}
