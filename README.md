# 🩸 RE-50 — Реверс-инжиниринг с нуля (десктоп-приложение)

Кроссплатформенное приложение курса на **Rust + egui/eframe 0.29**.
Работает на Linux, macOS, Windows 11.

## Возможности
- 📚 Полный курс: 7 модулей, 18 недель (лекции, лабы, problem sets, чек-пойнты)
- 🎯 Тренажёр-квиз: 31 вопрос с объяснениями, фильтр по модулям, статистика
- 🏅 15 ачивок + XP-система
- 📓 Журнал с автосохранением
- 🔗 Библиотека из 35 ресурсов
- Прогресс хранится локально: `~/.config/re50/progress.json` (Linux), `~/Library/Application Support/re50/` (macOS), `%APPDATA%/re50/` (Windows)

## Сборка
```bash
cargo build --release
./target/release/re50
```
Зависимости Linux: libxkbcommon-x11, стандартные GUI-библиотеки.

## Запуск готового бинаря (Linux x86-64)
```bash
chmod +x re50 && ./re50
```

## Структура
- `src/main.rs` — точка входа
- `src/curriculum.rs` — модель данных курса
- `src/state.rs` — прогресс, ачивки, XP, сохранение
- `src/ui.rs` — интерфейс (egui)
- `assets/curriculum.json` — весь контент курса (генерируется из исходного плана)
- `tools/gen_curriculum.py` — генератор curriculum.json
