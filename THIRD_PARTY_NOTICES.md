# Сторонние компоненты

RE-50 распространяется под лицензией MIT (см. [LICENSE](LICENSE)). Ниже — то, что вошло в бинарь и требует упоминания.

## Шрифты

| Шрифт | Где | Лицензия |
|---|---|---|
| **Noto Emoji** (v3.002, статический экземпляр `wght=400`) | `assets/fonts/NotoEmoji-Regular.ttf` | SIL Open Font License 1.1, текст — `assets/fonts/OFL.txt`. Copyright 2013 Google LLC |
| Ubuntu Light | входит в крейт `epaint_default_fonts` | Ubuntu Font Licence 1.0 |
| Hack | там же | MIT + Bitstream Vera |
| Noto Emoji (старая версия) | там же | SIL OFL 1.1 |
| emoji-icon-font | там же | MIT |

Noto Emoji подключён как запасной шрифт: в наборе egui нет эмодзи новее Unicode 11 (🩸 🧪 🥋 🧭 рисовались бы квадратами). Тест `every_character_has_a_glyph` следит, чтобы каждый символ курса и интерфейса имел глиф.

## Библиотеки Rust

Приложение собрано на [egui/eframe](https://github.com/emilk/egui) (MIT OR Apache-2.0), `serde` и `serde_json` (MIT OR Apache-2.0) и их зависимостях; полный список с версиями — в `Cargo.lock`. Для тестов используются `sha2` и `skrifa` (MIT OR Apache-2.0; `skrifa` уже входит в дерево egui). У каждого из примерно 400 пакетов есть разрешающая лицензия (MIT, Apache-2.0, BSD, Zlib, ISC, Unicode-3.0, BSL-1.0, Unlicense, 0BSD); где указано «OR LGPL/GPL», выбирается MIT или Apache-2.0.

Тексты лицензий и авторские права лежат в исходниках пакетов (у части крейтов egui файла лицензии в архиве нет — он есть в репозитории проекта). Полного пакета лицензий в этом репозитории нет: при публикации бинарей сформируйте его, например, командой `cargo about generate`, и приложите к релизу.

## Учебные бинари

Челленджи в `assets/challenges/` собраны из исходников `.c` в этом же каталоге (GCC для ELF, MinGW-w64 для PE) и не содержат чужого кода.
