//! Хранилище прогресса: пути по платформам, атомарная запись, бэкапы, восстановление.

use serde::{de::DeserializeOwned, Serialize};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "progress.json";
/// Сколько резервных копий хранить (`progress.json.bak1` … `bakN`).
pub const BACKUPS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    MacOs,
    Unix,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Unix
        }
    }
}

type Env<'a> = &'a dyn Fn(&str) -> Option<OsString>;

fn non_empty(v: Option<OsString>) -> Option<OsString> {
    v.filter(|s| !s.is_empty())
}

/// Каталог настроек приложения (без создания). `RE50_CONFIG_DIR` переопределяет платформенный.
pub fn config_dir_for(os: Os, env: Env) -> Option<PathBuf> {
    if let Some(dir) = non_empty(env("RE50_CONFIG_DIR")) {
        return Some(PathBuf::from(dir));
    }
    match os {
        Os::Windows => env("APPDATA").map(|d| PathBuf::from(d).join("re50")),
        Os::MacOs => env("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/re50")),
        Os::Unix => non_empty(env("XDG_CONFIG_HOME"))
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join("re50")),
    }
}

/// Корень для экспортов (профиль, журнал, лабы). `RE50_EXPORT_DIR` переопределяет домашний каталог.
pub fn export_dir_for(os: Os, env: Env) -> Option<PathBuf> {
    if let Some(dir) = non_empty(env("RE50_EXPORT_DIR")) {
        return Some(PathBuf::from(dir));
    }
    match os {
        Os::Windows => non_empty(env("USERPROFILE"))
            .or_else(|| {
                let mut home = env("HOMEDRIVE")?;
                home.push(env("HOMEPATH")?);
                Some(home)
            })
            .map(PathBuf::from),
        _ => non_empty(env("HOME")).map(PathBuf::from),
    }
}

/// Где приложение хранит данные и куда пишет экспорты.
#[derive(Clone, Debug, Default)]
pub struct Paths {
    pub config_dir: Option<PathBuf>,
    pub export_dir: Option<PathBuf>,
}

impl Paths {
    pub fn detect() -> Self {
        let env = |k: &str| std::env::var_os(k);
        Paths {
            config_dir: config_dir_for(Os::current(), &env),
            export_dir: export_dir_for(Os::current(), &env),
        }
    }

    /// Всё внутри `root` — для тестов и изолированных запусков.
    pub fn in_dir(root: &Path) -> Self {
        Paths {
            config_dir: Some(root.join("config")),
            export_dir: Some(root.join("home")),
        }
    }

    /// Без диска: прогресс не сохраняется, экспорт недоступен.
    pub fn none() -> Self {
        Paths::default()
    }

    pub fn progress_file(&self) -> Option<PathBuf> {
        self.config_dir.as_ref().map(|d| d.join(FILE_NAME))
    }

    pub fn export_root(&self) -> Result<&Path, String> {
        self.export_dir.as_deref().ok_or_else(|| {
            "не удалось определить домашнюю папку (задайте переменную RE50_EXPORT_DIR)".to_string()
        })
    }
}

/// Результат чтения: значение и, если пришлось восстанавливаться, сообщение для пользователя.
pub struct Loaded<T> {
    pub value: Option<T>,
    pub notice: Option<String>,
}

pub struct Storage {
    file: Option<PathBuf>,
    last_written: Option<String>,
}

impl Storage {
    pub fn new(file: Option<PathBuf>) -> Self {
        Storage {
            file,
            last_written: None,
        }
    }

    fn sibling(path: &Path, suffix: &str) -> PathBuf {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        path.with_file_name(format!("{name}{suffix}"))
    }

    /// Читает основной файл; при порче откладывает его в `.corrupt` и берёт свежий валидный бэкап.
    /// Исправный основной файл ротирует бэкапы (раз за запуск).
    pub fn load<T: DeserializeOwned>(&mut self) -> Loaded<T> {
        let Some(path) = self.file.clone() else {
            return Loaded {
                value: None,
                notice: None,
            };
        };
        let mut notice = None;
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<T>(&text) {
                Ok(value) => {
                    self.rotate_backups();
                    return Loaded {
                        value: Some(value),
                        notice: None,
                    };
                }
                Err(e) => {
                    let quarantine = Self::sibling(&path, ".corrupt");
                    let _ = std::fs::rename(&path, &quarantine);
                    notice = Some(format!(
                        "Файл прогресса повреждён ({e}); он сохранён как {}",
                        quarantine.display()
                    ));
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => notice = Some(format!("Не удалось прочитать прогресс: {e}")),
        }
        for i in 1..=BACKUPS {
            let backup = Self::sibling(&path, &format!(".bak{i}"));
            let Ok(text) = std::fs::read_to_string(&backup) else {
                continue;
            };
            if let Ok(value) = serde_json::from_str::<T>(&text) {
                let note = format!(
                    "Прогресс восстановлен из резервной копии {}",
                    backup.display()
                );
                let notice = Some(notice.map_or(note.clone(), |n| format!("{n}. {note}")));
                return Loaded {
                    value: Some(value),
                    notice,
                };
            }
        }
        Loaded {
            value: None,
            notice,
        }
    }

    /// Сдвигает `bak1..bakN` и кладёт текущий основной файл в `bak1`.
    pub fn rotate_backups(&self) {
        let Some(path) = &self.file else { return };
        if !path.exists() {
            return;
        }
        for i in (1..BACKUPS).rev() {
            let from = Self::sibling(path, &format!(".bak{i}"));
            if from.exists() {
                let _ = std::fs::rename(&from, Self::sibling(path, &format!(".bak{}", i + 1)));
            }
        }
        let _ = std::fs::copy(path, Self::sibling(path, ".bak1"));
    }

    /// Атомарная запись (tmp + rename). `Ok(false)` — содержимое не изменилось, диск не тронут.
    pub fn save<T: Serialize>(&mut self, value: &T) -> Result<bool, String> {
        let Some(path) = &self.file else {
            return Ok(false);
        };
        let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        if self.last_written.as_deref() == Some(json.as_str()) {
            return Ok(false);
        }
        let dir = path.parent().ok_or("некорректный путь к файлу прогресса")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = Self::sibling(path, ".tmp");
        let write = || -> std::io::Result<()> {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(json.as_bytes())?;
            f.sync_all()?;
            std::fs::rename(&tmp, path)
        };
        write().map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })?;
        self.last_written = Some(json);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    struct Doc {
        n: u32,
    }

    fn storage(dir: &TempDir) -> Storage {
        Storage::new(Some(dir.path().join(FILE_NAME)))
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = TempDir::new("roundtrip");
        assert!(storage(&dir).save(&Doc { n: 5 }).unwrap());
        let loaded = storage(&dir).load::<Doc>();
        assert_eq!(loaded.value, Some(Doc { n: 5 }));
        assert!(loaded.notice.is_none());
    }

    #[test]
    fn unchanged_content_is_not_rewritten() {
        let dir = TempDir::new("unchanged");
        let mut s = storage(&dir);
        assert!(s.save(&Doc { n: 1 }).unwrap());
        assert!(!s.save(&Doc { n: 1 }).unwrap());
        assert!(s.save(&Doc { n: 2 }).unwrap());
    }

    #[test]
    fn save_leaves_no_temp_file() {
        let dir = TempDir::new("notmp");
        storage(&dir).save(&Doc { n: 1 }).unwrap();
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![FILE_NAME.to_string()]);
    }

    #[test]
    fn corrupt_file_is_quarantined_and_backup_restored() {
        let dir = TempDir::new("corrupt");
        let mut s = storage(&dir);
        s.save(&Doc { n: 1 }).unwrap();
        s.load::<Doc>(); // ротация: bak1 = {n:1}
        s.save(&Doc { n: 2 }).unwrap();
        std::fs::write(dir.path().join(FILE_NAME), "{ битый json").unwrap();

        let loaded = storage(&dir).load::<Doc>();
        assert_eq!(loaded.value, Some(Doc { n: 1 }));
        let notice = loaded
            .notice
            .expect("пользователь должен узнать о восстановлении");
        assert!(
            notice.contains("повреждён") && notice.contains("bak1"),
            "{notice}"
        );
        assert!(dir.path().join("progress.json.corrupt").exists());
    }

    #[test]
    fn backups_rotate_and_are_capped() {
        let dir = TempDir::new("rotate");
        for n in 1..=8u32 {
            let mut s = storage(&dir);
            s.load::<Doc>();
            s.save(&Doc { n }).unwrap();
        }
        let read = |name: &str| -> Doc {
            serde_json::from_str(&std::fs::read_to_string(dir.path().join(name)).unwrap()).unwrap()
        };
        assert_eq!(read("progress.json"), Doc { n: 8 });
        assert_eq!(read("progress.json.bak1"), Doc { n: 7 });
        assert_eq!(read("progress.json.bak5"), Doc { n: 3 });
        assert!(!dir.path().join("progress.json.bak6").exists());
    }

    #[test]
    fn missing_everything_is_a_quiet_fresh_start() {
        let dir = TempDir::new("fresh");
        let loaded = storage(&dir).load::<Doc>();
        assert!(loaded.value.is_none() && loaded.notice.is_none());
    }

    #[test]
    fn storage_without_path_is_a_noop() {
        let mut s = Storage::new(None);
        assert_eq!(s.save(&Doc { n: 1 }), Ok(false));
        assert!(s.load::<Doc>().value.is_none());
    }

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| {
            map.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| OsString::from(v))
        }
    }

    #[test]
    fn config_dir_follows_platform_conventions() {
        let win = env_of(&[("APPDATA", r"C:\Users\a\AppData\Roaming")]);
        assert_eq!(
            config_dir_for(Os::Windows, &win),
            Some(PathBuf::from(r"C:\Users\a\AppData\Roaming").join("re50"))
        );
        let mac = env_of(&[("HOME", "/Users/a")]);
        assert_eq!(
            config_dir_for(Os::MacOs, &mac),
            Some(PathBuf::from("/Users/a/Library/Application Support/re50"))
        );
        let xdg = env_of(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/home/a")]);
        assert_eq!(
            config_dir_for(Os::Unix, &xdg),
            Some(PathBuf::from("/x/re50"))
        );
        let home = env_of(&[("XDG_CONFIG_HOME", ""), ("HOME", "/home/a")]);
        assert_eq!(
            config_dir_for(Os::Unix, &home),
            Some(PathBuf::from("/home/a/.config/re50"))
        );
        let over = env_of(&[("RE50_CONFIG_DIR", "/custom"), ("HOME", "/home/a")]);
        assert_eq!(
            config_dir_for(Os::Unix, &over),
            Some(PathBuf::from("/custom"))
        );
        assert_eq!(config_dir_for(Os::Unix, &env_of(&[])), None);
    }

    #[test]
    fn export_dir_uses_userprofile_on_windows() {
        // регресс: раньше на Windows экспорт уходил в «.» (переменной HOME там нет)
        let win = env_of(&[("USERPROFILE", r"C:\Users\a")]);
        assert_eq!(
            export_dir_for(Os::Windows, &win),
            Some(PathBuf::from(r"C:\Users\a"))
        );
        let drive = env_of(&[("HOMEDRIVE", "C:"), ("HOMEPATH", r"\Users\a")]);
        assert_eq!(
            export_dir_for(Os::Windows, &drive),
            Some(PathBuf::from(r"C:\Users\a"))
        );
        assert_eq!(
            export_dir_for(Os::Unix, &env_of(&[("HOME", "/home/a")])),
            Some(PathBuf::from("/home/a"))
        );
        assert_eq!(export_dir_for(Os::Windows, &env_of(&[])), None);
    }
}
