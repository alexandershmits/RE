"""Тесты вспомогательных скриптов: без сети, gcc нужен только генератору. Запуск:
python3 -m unittest discover -s tools -p "test_*.py" -v
"""
import ast
import json
import os
import re
import shutil
import sys
import tempfile
import unittest
import urllib.error
from unittest import mock

TOOLS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(TOOLS, "..")
sys.path.insert(0, TOOLS)

import challenge_generator  # noqa: E402
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


if __name__ == "__main__":
    unittest.main()
