#!/usr/bin/env python3
"""Проверка внешних ссылок курса (resources и все URL в тексте curriculum.json).

Классификация ответа:
  OK       — HTTP < 400
  BLOCKED  — 401/403/429/999: сайт отказывает ботам, ссылку это не порочит
  FLAKY    — таймаут, 5xx, обрыв соединения, временный сбой DNS: перепроверьте позже
  DEAD     — 404/410 или домена не существует (NXDOMAIN): ссылку нужно чинить
Код возврата 1 только при DEAD.

Использование: python3 tools/check_links.py [--json путь] [--timeout секунд]
"""
import concurrent.futures
import http.client
import json
import os
import re
import socket
import sys
import urllib.error
import urllib.request

for stream in (sys.stdout, sys.stderr):  # кириллица в отчёте не должна ронять вывод в консоли Windows
    if hasattr(stream, "reconfigure"):
        stream.reconfigure(encoding="utf-8")

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
URL_RE = re.compile(r"https?://[^\s)\]»\"'<>]+")
UA = "Mozilla/5.0 (compatible; re50-link-checker; +https://github.com/alexandershmits/RE)"


def collect(node, found):
    if isinstance(node, str):
        found.update(u.rstrip(".,;:") for u in URL_RE.findall(node))
    elif isinstance(node, list):
        for x in node:
            collect(x, found)
    elif isinstance(node, dict):
        for x in node.values():
            collect(x, found)


def probe(url, timeout):
    """Возвращает (класс, детали)."""
    last = "нет ответа"
    for method in ("HEAD", "GET"):
        req = urllib.request.Request(url, method=method, headers={"User-Agent": UA})
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return "OK", str(resp.status)
        except urllib.error.HTTPError as e:
            if e.code in (404, 410):
                return "DEAD", str(e.code)
            if e.code in (401, 403, 429, 999):
                return "BLOCKED", str(e.code)
            last = str(e.code)  # HEAD часто не поддерживается — пробуем GET
        except urllib.error.URLError as e:
            # EAI_AGAIN (нет сети, сбой резолвера) — не приговор ссылке, это FLAKY
            if isinstance(e.reason, socket.gaierror) and e.reason.errno == socket.EAI_NONAME:
                return "DEAD", f"домен не найден: {e.reason}"
            last = str(e.reason)
        except (TimeoutError, socket.timeout, ConnectionError, OSError, http.client.HTTPException) as e:
            last = str(e)
        except ValueError as e:  # некорректный URL в тексте курса: ссылку надо чинить, а не ронять весь прогон
            return "DEAD", f"некорректный URL: {e}"
    return "FLAKY", last


def main():
    args = sys.argv[1:]
    timeout = int(args[args.index("--timeout") + 1]) if "--timeout" in args else 20
    path = args[args.index("--json") + 1] if "--json" in args else os.path.join(ROOT, "assets", "curriculum.json")
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    urls = set()
    collect(data, urls)
    with concurrent.futures.ThreadPoolExecutor(8) as pool:
        results = dict(zip(sorted(urls), pool.map(lambda u: probe(u, timeout), sorted(urls))))
    order = {"DEAD": 0, "FLAKY": 1, "BLOCKED": 2, "OK": 3}
    for url, (kind, detail) in sorted(results.items(), key=lambda kv: (order[kv[1][0]], kv[0])):
        print(f"{kind:8} {detail:>28}  {url}")
    dead = [u for u, (k, _) in results.items() if k == "DEAD"]
    print(f"\nвсего {len(results)}: " + ", ".join(f"{k}={sum(1 for v in results.values() if v[0] == k)}" for k in order))
    if dead:
        print(f"::error::мёртвых ссылок: {len(dead)}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
