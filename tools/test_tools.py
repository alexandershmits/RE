"""Тесты вспомогательных скриптов: без сети, gcc нужен только генератору. Запуск:
python3 -m unittest discover -s tools -p "test_*.py" -v
"""
import ast
import contextlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import urllib.error
from unittest import mock

TOOLS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(TOOLS, "..")
sys.path.insert(0, TOOLS)

import challenge_generator  # noqa: E402
import changelog_section  # noqa: E402
import check_links  # noqa: E402


def load(name):
    with open(os.path.join(ROOT, "assets", name), encoding="utf-8") as f:
        return json.load(f)


class CheckLinks(unittest.TestCase):
    def test_urls_are_collected_from_nested_text_without_trailing_punctuation(self):
        found = set()
        check_links.collect({"a": ["см. https://a.io/x, и (http://b.io/y)."], "b": {"c": "нет ссылок"}}, found)
        self.assertEqual(found, {"https://a.io/x", "http://b.io/y"})

    def probe(self, error):
        with mock.patch("urllib.request.urlopen", side_effect=error):
            return check_links.probe("https://example.invalid/", 1)[0]

    def http_error(self, code):
        return urllib.error.HTTPError("https://x", code, "x", {}, None)

    def test_classification(self):
        import socket
        self.assertEqual(self.probe(self.http_error(404)), "DEAD")
        self.assertEqual(self.probe(self.http_error(403)), "BLOCKED")
        self.assertEqual(self.probe(self.http_error(503)), "FLAKY")
        self.assertEqual(self.probe(TimeoutError()), "FLAKY")
        nxdomain = urllib.error.URLError(socket.gaierror(socket.EAI_NONAME, "нет такого домена"))
        self.assertEqual(self.probe(nxdomain), "DEAD")
        # нет сети или сбой резолвера — не приговор ссылке
        offline = urllib.error.URLError(socket.gaierror(socket.EAI_AGAIN, "временный сбой"))
        self.assertEqual(self.probe(offline), "FLAKY")

    def test_odd_failures_do_not_crash_the_run(self):
        import http.client
        self.assertEqual(self.probe(http.client.RemoteDisconnected("обрыв")), "FLAKY")
        self.assertEqual(self.probe(ValueError("bad url")), "DEAD")


class Drills(unittest.TestCase):
    def test_python_reference_answers_are_valid_python(self):
        checked = 0
        for drill in load("drills.json")["script"]:
            answer = drill["answer"]
            if drill["task"].startswith("Python") and not re.search("[А-Яа-яЁё]", answer):
                ast.parse(answer)  # SyntaxError = эталон нельзя запустить
                checked += 1
        self.assertGreaterEqual(checked, 5)


@unittest.skipUnless(shutil.which("gcc") and sys.platform.startswith("linux"), "нужны gcc и Linux")
class Generator(unittest.TestCase):
    def test_every_template_solves_itself_for_several_seeds(self):
        with tempfile.TemporaryDirectory() as out:
            for template in challenge_generator.TEMPLATES:
                for seed in range(3):
                    meta = challenge_generator.build(template, out, seed=seed)
                    self.assertIn("flag", meta)

    def test_self_solve_rejects_a_wrong_answer_and_a_degenerate_binary(self):
        original = challenge_generator.BUILDERS["gen3"]

        def wrong_answer(rng):
            params, hint, flag, answer, wrong = original(rng)
            return params, hint, flag, [answer[0] + "!"], wrong

        with tempfile.TemporaryDirectory() as out:
            with mock.patch.dict(challenge_generator.BUILDERS, {"gen3": wrong_answer}):
                with self.assertRaisesRegex(RuntimeError, "не выдал флаг"):
                    challenge_generator.build("gen3", out, seed=1)

            original_gen2 = challenge_generator.BUILDERS["gen2"]

            def prints_flag_always(rng):
                params, hint, flag, answer, _ = original_gen2(rng)
                return params, hint, flag, answer, answer  # «неверный» ввод на деле верный

            with mock.patch.dict(challenge_generator.BUILDERS, {"gen2": prints_flag_always}):
                with self.assertRaisesRegex(RuntimeError, "неверном вводе"):
                    challenge_generator.build("gen2", out, seed=1)


@unittest.skipUnless(shutil.which("bash"), "нужен bash")
class ReleaseTag(unittest.TestCase):
    SCRIPT = os.path.join(TOOLS, "check_release_tag.sh")

    def check(self, tag, version):
        return subprocess.run(["bash", self.SCRIPT, tag, version], capture_output=True, text=True)

    def test_two_and_three_part_tags_mean_the_same_version(self):
        # прежние теги проекта — v7.1, v6.9; Cargo требует три числа
        for tag in ("v7.2", "v7.2.0"):
            result = self.check(tag, "7.2.0")
            self.assertEqual(result.returncode, 0, f"{tag}: {result.stdout}")
        self.assertEqual(self.check("v7.10", "7.10.0").returncode, 0)
        self.assertEqual(self.check("v7.2.1", "7.2.1").returncode, 0)

    def test_everything_else_is_rejected_with_a_github_error(self):
        wrong = [
            ("v7.2", "7.2.1"),  # двухчастный тег — это X.Y.0
            ("v7.2.1", "7.2.0"),
            ("v7.3", "7.2.0"),
            ("v7", "7.2.0"),
            ("7.2", "7.2.0"),  # без «v» workflow не запустится, но скрипт тоже не должен соглашаться
            ("v7.2.0-rc1", "7.2.0"),
            ("v7.2-beta", "7.2.0"),
            ("v7.2.0.0", "7.2.0"),
            ("v07.2", "7.2.0"),
            ("vx.y", "7.2.0"),
            ("", "7.2.0"),
            ("v7.2", ""),  # cargo pkgid ничего не вернул
        ]
        for tag, version in wrong:
            with self.subTest(tag=tag, version=version):
                result = self.check(tag, version)
                self.assertEqual(result.returncode, 1, result.stdout)
                self.assertIn("::error::", result.stdout)


class Changelog(unittest.TestCase):
    SAMPLE = """# Changelog

## [2.0.0] — 2026-02-02

### Добавлено
- б

## [1.0.0] — 2026-01-01
- а
"""

    def run_main(self, text, version="2.0.0"):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "CHANGELOG.md")
            with open(path, "w", encoding="utf-8") as f:
                f.write(text)
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = changelog_section.main(["changelog_section.py", version, path])
            return code, out.getvalue(), err.getvalue()

    def test_a_section_is_cut_at_the_next_release_and_has_no_heading(self):
        self.assertEqual(
            changelog_section.section(self.SAMPLE, "2.0.0"), ("2026-02-02", "### Добавлено\n- б")
        )
        self.assertEqual(changelog_section.section(self.SAMPLE, "1.0.0"), ("2026-01-01", "- а"))
        self.assertEqual(changelog_section.section(self.SAMPLE, "3.0.0"), (None, None))

    def test_release_text_is_printed(self):
        code, out, _ = self.run_main(self.SAMPLE)
        self.assertEqual((code, out), (0, "### Добавлено\n- б\n"))

    def test_an_unreleasable_section_is_refused(self):
        cases = {
            "нет раздела": (self.SAMPLE, "9.9.9", "нет раздела"),
            "без даты": (self.SAMPLE.replace(" — 2026-02-02", ""), "2.0.0", "нет даты"),
            "дата не по формату": (self.SAMPLE.replace("2026-02-02", "2026-2-2"), "2.0.0", "нет даты"),
            "несуществующий день": (self.SAMPLE.replace("2026-02-02", "2026-02-31"), "2.0.0", "нет даты"),
            "пустой раздел": ("## [2.0.0] — 2026-02-02\n\n## [1.0.0] — 2026-01-01\n- а\n", "2.0.0", "пуст"),
        }
        for name, (text, version, message) in cases.items():
            with self.subTest(name):
                code, out, err = self.run_main(text, version)
                self.assertEqual((code, out), (1, ""), err)
                self.assertIn(message, err)

    def test_the_current_version_has_a_dated_section_and_it_matches_cargo(self):
        # без этого релиз упал бы в самом конце, после сборки трёх бинарей
        version = changelog_section.cargo_version()
        self.assertRegex(version, r"^\d+\.\d+\.\d+$", "Cargo требует три числа")
        with open(os.path.join(ROOT, "CHANGELOG.md"), encoding="utf-8") as f:
            date, body = changelog_section.section(f.read(), version)
        self.assertTrue(changelog_section.valid_date(date), f"у раздела {version} нет настоящей даты: {date!r}")
        self.assertTrue(body, f"раздел {version} пуст")
        self.assertNotIn("\n## [", "\n" + body)


if __name__ == "__main__":
    unittest.main()
