#!/usr/bin/env python3
"""Раздел CHANGELOG.md для версии — текст релиза на GitHub.

Использование: changelog_section.py [версия] [путь к CHANGELOG.md]
Без версии берётся `version` из Cargo.toml. Код возврата 1, если раздела нет, у него нет настоящей даты
`ГГГГ-ММ-ДД` или он пуст: такой релиз выпускать нельзя.
"""
import datetime
import os
import re
import sys

for stream in (sys.stdout, sys.stderr):  # кириллица в тексте не должна ронять вывод в консоли Windows
    if hasattr(stream, "reconfigure"):
        stream.reconfigure(encoding="utf-8")

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
HEADING = re.compile(r"^## \[(?P<version>[^\]]+)\](?: — (?P<date>\S+))?\s*$")


def cargo_version(path=None):
    """Версия пакета из Cargo.toml (первая строка `version = "…"` в начале строки)."""
    with open(path or os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as f:
        match = re.search(r'^version\s*=\s*"([^"]+)"', f.read(), re.MULTILINE)
    if not match:
        raise ValueError("в Cargo.toml нет version")
    return match.group(1)


def section(text, version):
    """(дата, тело) раздела `## [версия] — дата` без заголовка; (None, None), если раздела нет."""
    lines = text.splitlines()
    for start, line in enumerate(lines):
        match = HEADING.match(line)
        if match and match["version"] == version:
            end = next(
                (i for i in range(start + 1, len(lines)) if lines[i].startswith("## [")),
                len(lines),
            )
            return match["date"], "\n".join(lines[start + 1 : end]).strip()
    return None, None


def valid_date(value):
    """Только `ГГГГ-ММ-ДД` и только существующий день."""
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", value or ""):
        return False
    try:
        datetime.date.fromisoformat(value)
    except ValueError:
        return False
    return True


def main(argv):
    version = argv[1] if len(argv) > 1 else cargo_version()
    path = argv[2] if len(argv) > 2 else os.path.join(ROOT, "CHANGELOG.md")
    with open(path, encoding="utf-8") as f:
        date, body = section(f.read(), version)
    if body is None:
        print(f"::error::в CHANGELOG.md нет раздела для версии {version}", file=sys.stderr)
        return 1
    if not valid_date(date):
        print(f"::error::у раздела {version} в CHANGELOG.md нет даты ГГГГ-ММ-ДД (сейчас: {date!r})", file=sys.stderr)
        return 1
    if not body:
        print(f"::error::раздел {version} в CHANGELOG.md пуст", file=sys.stderr)
        return 1
    print(body)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
