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

Приложение собрано на [egui/eframe](https://github.com/emilk/egui) (MIT OR Apache-2.0), `serde` и `serde_json` (MIT OR Apache-2.0) и их зависимостях; полный список с версиями — в `Cargo.lock`. Для тестов используются `sha2` и `ttf-parser` (MIT OR Apache-2.0).

## Учебные бинари

Челленджи в `assets/challenges/` собраны из исходников `.c` в этом же каталоге (GCC для ELF, MinGW-w64 для PE) и не содержат чужого кода.
