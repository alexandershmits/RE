#!/usr/bin/env python3
"""Проверка челленджей: бинари <-> исходники <-> assets/curriculum.json.

Что проверяется:
  1. SHA-256 (первые 16 hex-символов) ELF и PE совпадают с curriculum.json.
  2. Поставляемый ELF на верном вводе (tools/challenge_solutions.json) печатает FLAG{...}
     из curriculum.json, а на неверном — нет (только Linux x86-64).
  3. --rebuild: каждый .c собирается gcc (и mingw для .exe, если он установлен) и ведёт себя так же.

Использование: python3 tools/verify_challenges.py [--rebuild]
Код возврата 0 — всё в порядке.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
ASSETS = os.path.join(ROOT, "assets")
CHALLENGES = os.path.join(ASSETS, "challenges")


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def sha_prefix(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()[:16]


def run(binary, lines):
    result = subprocess.run(
        [binary], input="\n".join(lines) + "\n", capture_output=True, text=True, timeout=10
    )
    return result.stdout


def behaves(binary, flag, solution):
    marker = f"FLAG{{{flag}}}"
    return marker in run(binary, solution) and marker not in run(binary, ["zzzzzz"] * 3)


def main():
    rebuild = "--rebuild" in sys.argv[1:]
    curriculum = load(os.path.join(ASSETS, "curriculum.json"))
    solutions = load(os.path.join(ROOT, "tools", "challenge_solutions.json"))
    can_run = sys.platform.startswith("linux")
    problems = []

    for ch in curriculum["challenges"]:
        cid, flag = ch["id"], ch["flag"]
        elf = os.path.join(ASSETS, ch["file"])
        exe = os.path.join(ASSETS, ch["file_exe"])
        if sha_prefix(elf) != ch["sha256"]:
            problems.append(f"{cid}: SHA-256 ELF не совпадает с curriculum.json")
        if sha_prefix(exe) != ch["sha256_exe"]:
            problems.append(f"{cid}: SHA-256 EXE не совпадает с curriculum.json")
        if cid not in solutions:
            problems.append(f"{cid}: нет решения в tools/challenge_solutions.json")
            continue
        if can_run and not behaves(elf, flag, solutions[cid]):
            problems.append(f"{cid}: поставляемый ELF не даёт флаг на верном вводе или пускает неверный")

    if rebuild:
        mingw = shutil.which("x86_64-w64-mingw32-gcc")
        with tempfile.TemporaryDirectory() as tmp:
            for ch in curriculum["challenges"]:
                cid = ch["id"]
                source = os.path.join(CHALLENGES, cid + ".c")
                out = os.path.join(tmp, cid)
                build = subprocess.run(["gcc", "-O0", "-w", source, "-o", out], capture_output=True, text=True)
                if build.returncode != 0:
                    problems.append(f"{cid}: gcc не собрал {cid}.c: {build.stderr.strip()[:200]}")
                elif can_run and not behaves(out, ch["flag"], solutions[cid]):
                    problems.append(f"{cid}: пересобранный из .c бинарь ведёт себя иначе")
                if mingw:
                    exe = os.path.join(tmp, cid + ".exe")
                    build = subprocess.run([mingw, "-O0", "-w", source, "-o", exe], capture_output=True, text=True)
                    header = b""
                    if build.returncode == 0 and os.path.exists(exe):
                        with open(exe, "rb") as f:
                            header = f.read(2)
                    if header != b"MZ":
                        problems.append(f"{cid}: mingw не собрал корректный PE из {cid}.c: {build.stderr.strip()[:200]}")

    # генератор «adversarial loop» из приложения: флаг каждого варианта обязан лежать в собранном бинаре
        with tempfile.TemporaryDirectory() as gen:
            made = subprocess.run(
                [sys.executable, os.path.join(ROOT, "tools", "challenge_generator.py"), "all", gen],
                capture_output=True, text=True,
            )
            if made.returncode != 0:
                problems.append(f"генератор не отработал: {made.stderr.strip()[:200]}")
            else:
                for name in sorted(n for n in os.listdir(gen) if n.endswith(".meta.json")):
                    meta = load(os.path.join(gen, name))
                    with open(os.path.join(gen, meta["id"]), "rb") as f:
                        if f"FLAG{{{meta['flag']}}}".encode() not in f.read():
                            problems.append(f"генератор {meta['id']}: флага нет в бинаре")

    checked = len(curriculum["challenges"])
    if problems:
        print(f"НАЙДЕНО ПРОБЛЕМ: {len(problems)}")
        for p in problems:
            print(" -", p)
        return 1
    mode = "хеши, поведение ELF" + (", пересборка из .c" if rebuild else "")
    print(f"OK: {checked} челленджей проверено ({mode}{'' if can_run else '; запуск пропущен — не Linux'})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
