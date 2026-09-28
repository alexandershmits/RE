#!/usr/bin/env python3
"""Генерирует curriculum.json для RE-50 из исходного .md курса + добавляет PSet0, квизы, ачивки."""
import re, json

SRC = '/home/agentuser/RE-50-курс-реверс-инжиниринга.md'
OUT = '/home/agentuser/re50-app/assets/curriculum.json'
src = open(SRC, encoding='utf-8').read()

# ---------- helper: slice section between two headers ----------
def slice_between(start_pat, end_pat, text=src):
    s = re.search(start_pat, text)
    if not s: return ""
    e = re.search(end_pat, text[s.end():])
    return text[s.end(): s.end() + e.start()] if e else text[s.end():]

MODULE_NAMES = {
    0: ("Модуль 0 — Ориентация", "🧭"),
    1: ("Модуль 1. Фундамент", "🧱"),
    2: ("Модуль 2. Инструменты", "🔧"),
    3: ("Модуль 3. Практика — Crackmes", "🥊"),
    4: ("Модуль 4. Систематика", "🎓"),
    5: ("Модуль 5. Реальный мир", "🌍"),
    6: ("Модуль 6. Реверс через ИИ", "🤖"),
    7: ("Финал", "🏆"),
}

def module_of_week(w):
    if w == 0: return 0
    if w <= 6: return 1
    if w <= 9: return 2
    if w <= 14: return 3
    if w <= 17: return 4
    if w <= 20: return 5
    if w <= 22: return 6
    return 7

# ---------- parse weeks + content blocks ----------
week_ranges = []
for m in re.finditer(r'## Неделя (\d+)(?:–(\d+))?\. (.+)', src):
    s = int(m.group(1)); e = int(m.group(2)) if m.group(2) else s
    week_ranges.append((s, e, m.group(3).strip()))

# lecture topics per week (heading + bullet paragraphs up to next heading)
def blocks_for(text):
    """returns dict with lectures, lab, pset, checkpoint lists from a week's text"""
    out = {"lectures": [], "lab": "", "lab_steps": [], "psets": [], "checkpoint": []}
    cur = None
    for line in text.splitlines():
        if line.startswith('### 📖'):
            cur = 'lec'; continue
        if line.startswith('### 🧪'):
            out['lab'] = line.replace('### 🧪', '').strip(); cur = 'lab_steps'; continue
        if line.startswith('### 📝'):
            out['psets'].append(line.replace('### 📝', '').strip()); cur = None; continue
        if line.startswith('### ✅'):
            cur = 'chk'; continue
        if line.startswith('#') and cur:
            cur = None
        if cur == 'lec' and line.startswith('- '):
            out['lectures'].append(line[2:].strip())
        elif cur == 'lab_steps' and re.match(r'^\d+\.', line):
            out['lab_steps'].append(re.sub(r'^\d+\.\s*', '', line).strip())
        elif cur == 'chk' and line.startswith('- [ ]'):
            out['checkpoint'].append(line[5:].strip())
    return out

weeks = []
for (s, e, title) in week_ranges:
    # slice text of this week: from its header to next '## Неделя' or '# ' module/final header
    pat = re.compile(r'## Неделя %d(?:–%d)?\. %s' % (s, e, re.escape(title)))
    sm = pat.search(src)
    nxt = re.search(r'\n## Неделя |\n# ', src[sm.end():])
    body = src[sm.end(): sm.end() + nxt.start()] if nxt else src[sm.end():]
    b = blocks_for(body)
    weeks.append({
        "id": f"w{s}" if s == e else f"w{s}-{e}",
        "num": s,
        "num_end": e,
        "title": title,
        "module": module_of_week(s),
        "lectures": b['lectures'],
        "lab": {"title": b['lab'], "steps": b['lab_steps']} if b['lab'] else None,
        "psets": b['psets'],
        "checkpoint": b['checkpoint'],
    })

# ---------- PSet0 (week 0) ----------
week0 = {
    "id": "w0", "num": 0, "num_end": 0,
    "title": "Ориентация и настройка лаборатории",
    "module": 0,
    "lectures": [
        "Прочитать курс целиком, понять маршрут (24 недели)",
        "Собрать лабораторию: VirtualBox + Windows VM (Ghidra, x64dbg, DIE, HxD, Scylla)",
        "Сделать снапшот ВМ с именем CLEAN",
        "Создать репозиторий re50-journal — журнал всех работ",
        "Прочитать RE-MA-Roadmap — увидеть карту от нуля до malware-аналитика"
    ],
    "lab": {"title": "Лаборатория готова?", "steps": [
        "Установить VirtualBox и Windows 10/11 evaluation VM",
        "Поставить в ВМ: Ghidra, x64dbg, Detect It Easy, HxD, Scylla",
        "Сделать снапшот CLEAN сразу после настройки",
        "На хосте: Python 3, git, VS Code",
    ]},
    "psets": ["PSet0: ВМ со снапшотом CLEAN + репозиторий журнала с README (кто вы, зачем курс, цель через 6 месяцев)"],
    "checkpoint": [
        "ВМ запускается, снапшот CLEAN сделан",
        "Репозиторий журнала создан",
        "Я знаю маршрут курса и правила честности"
    ],
}

# ---------- Финал (week 23-24) ----------
final_week = {
    "id": "w23-24", "num": 23, "num_end": 24,
    "title": "Финальный проект + защита",
    "module": 7,
    "lectures": [
        "Вариант A. Аналитик: полный анализ нового сэмпла (триаж → распаковка → статика+динамика → YARA + IOC + отчёт)",
        "Вариант B. Крэкер: 3 crackmes 3★, для каждого — ключген + write-up с алгоритмом",
        "Вариант C. Инженер: 3 собственных crackmes-задачи с решениями",
        "Вариант D. AI-исследователь: пайплайн бинарь → агент с MCP → отчёт с системой валидации",
        "Защита: репозиторий всех 18 PSets + эссе «Мой пайплайн RE-анализа» + публичный write-up"
    ],
    "lab": {"title": "Подготовка к защите", "steps": [
        "Выбрать один вариант финального проекта",
        "Составить план работ на 2 недели",
        "Подготовить репозиторий: журнал PSets, write-ups, финальный проект",
    ]},
    "psets": ["Финальный проект: A/B/C/D + эссе «Мой пайплайн RE-анализа» + публичный write-up"],
    "checkpoint": [
        "Все 18 PSets сданы и задокументированы",
        "Финальный проект завершён и оформлен",
        "Эссе написано — мой личный чек-лист пайплайна",
    ],
}

weeks = [week0] + weeks + [final_week]

# ---------- resources (from исходного плана) ----------
resources = [
    {"name": "beginners.re — RE for Beginners (бесплатно)", "url": "https://beginners.re/", "category": "Книги"},
    {"name": "CS50x Harvard (недели про C)", "url": "https://cs50.harvard.edu/x/", "category": "Курсы"},
    {"name": "OpenSecurityTraining2 (Arch1001 и др.)", "url": "https://p.ost2.fyi/", "category": "Курсы"},
    {"name": "pwn.college (ASU, автопроверка)", "url": "https://pwn.college/", "category": "Курсы"},
    {"name": "Compiler Explorer (godbolt)", "url": "https://godbolt.org/", "category": "Инструменты"},
    {"name": "Ghidra (NSA)", "url": "https://ghidra-sre.org/", "category": "Инструменты"},
    {"name": "x64dbg", "url": "https://x64dbg.com/", "category": "Инструменты"},
    {"name": "Detect It Easy", "url": "https://github.com/horsicq/Detect-It-Easy", "category": "Инструменты"},
    {"name": "HxD hex-редактор", "url": "https://mh-nexus.de/hxd/", "category": "Инструменты"},
    {"name": "crackmes.one", "url": "https://crackmes.one/", "category": "Практика"},
    {"name": "ROP Emporium", "url": "https://ropemporium.com/", "category": "Практика"},
    {"name": "Felix Cloutier x86 reference", "url": "https://www.felixcloutier.com/x86/", "category": "Справочники"},
    {"name": "PE-формат (Microsoft)", "url": "https://learn.microsoft.com/windows/win32/debug/pe-format", "category": "Справочники"},
    {"name": "FLOSS (FLARE)", "url": "https://github.com/mandiant/flare-floss", "category": "Инструменты"},
    {"name": "capa (FLARE)", "url": "https://github.com/mandiant/capa", "category": "Инструменты"},
    {"name": "Scylla (IAT rebuild)", "url": "https://github.com/NtQuery/Scylla", "category": "Инструменты"},
    {"name": "Frida", "url": "https://frida.re/", "category": "Инструменты"},
    {"name": "FakeNet-NG", "url": "https://github.com/mandiant/flare-fakenet-ng", "category": "Инструменты"},
    {"name": "CAPEv2 sandbox", "url": "https://github.com/kevoreilly/capev2", "category": "Инструменты"},
    {"name": "Speakeasy", "url": "https://github.com/mandiant/speakeasy", "category": "Инструменты"},
    {"name": "PMAT-курс (HuskyHacks)", "url": "https://github.com/HuskyHacks/PMAT-course", "category": "Курсы"},
    {"name": "RE-MA-Roadmap", "url": "https://github.com/x86byte/RE-MA-Roadmap", "category": "Роадмапы"},
    {"name": "«Реверсинг с IDA Pro от 0» (перевод Нарвахи, RU)", "url": "https://yutewiyof.gitbook.io/intro-rev-ida-pro", "category": "Книги"},
    {"name": "Awesome-Reversing", "url": "https://github.com/ReversingID/Awesome-Reversing", "category": "Каталоги"},
    {"name": "GhidraMCP (LaurieWired)", "url": "https://github.com/LaurieWired/GhidraMCP", "category": "AI-реверс"},
    {"name": "ghidra-mcp (200+ тулов)", "url": "https://github.com/bethington/ghidra-mcp", "category": "AI-реверс"},
    {"name": "ida-pro-mcp (автор x64dbg)", "url": "https://github.com/mrexodia/ida-pro-mcp", "category": "AI-реверс"},
    {"name": "reverse-skill (роутер скиллов)", "url": "https://github.com/zhaoxuya520/reverse-skill", "category": "AI-реверс"},
    {"name": "SkillSpector (NVIDIA, сканер скиллов)", "url": "https://github.com/NVIDIA/SkillSpector", "category": "AI-реверс"},
    {"name": "awesome-ai-reverse", "url": "https://github.com/DiscoverBox/awesome-ai-reverse", "category": "AI-реверс"},
    {"name": "Awesome-RE-MCP", "url": "https://github.com/crowdere/Awesome-RE-MCP", "category": "AI-реверс"},
    {"name": "ctftime.org", "url": "https://ctftime.org/", "category": "Практика"},
    {"name": "OALabs (YouTube)", "url": "https://www.youtube.com/@OALabs", "category": "Видео"},
    {"name": "LiveOverflow (YouTube)", "url": "https://www.youtube.com/@LiveOverflow", "category": "Видео"},
    {"name": "Gynvael Coldwind (YouTube)", "url": "https://www.youtube.com/@GynvaelColdwind", "category": "Видео"},
]

# ---------- quizzes: тренажёр ----------
quizzes = [
  # Модуль 1 — C
  {"id":"q_c1","week":"w1-2","module":1,"question":"Что выведет sizeof для struct { char c; int i; } на x86-64 (типичное выравнивание)?","answers":["5","8","4","9"],"correct":1,"explain":"Компилятор вставляет 3 байта padding после char, чтобы int был выровнен — итого 8."},
  {"id":"q_c2","week":"w1-2","module":1,"question":"Что НЕВЕРНО насчёт char *p = &arr[3]; ?","answers":["p указывает на 4-й элемент массива","*(p+1) — это arr[4]","p - arr == 3","p содержит копию значения arr[3]"],"correct":3,"explain":"Указатель хранит АДРЕС, а не копию значения. Всё остальное верно."},
  {"id":"q_c3","week":"w1-2","module":1,"question":"Почему return &local_var; — баг?","answers":["Локальная переменная лежит в куче","Локальная переменная в стеке умирает при выходе из функции","Так нельзя возвращать числа","Это UB только с флагом -O2"],"correct":1,"explain":"Стековый кадр функции уничтожается при выходе — указатель станет висячим (dangling pointer)."},
  {"id":"q_c4","week":"w1-2","module":1,"question":"Строка \"ABC\" в C занимает в памяти:","answers":["3 байта","4 байта","8 байт","зависит от оптимизации"],"correct":1,"explain":"Нуль-терминатор \\0 добавляет 4-й байт."},
  # Модуль 1 — asm
  {"id":"q_a1","week":"w3-4","module":1,"question":"eax — это какие биты регистра rax?","answers":["Старшие 32","Младшие 32","Младшие 16","Весь регистр"],"correct":1,"explain":"rax(64) → eax(младшие 32) → ax(16) → al(8)."},
  {"id":"q_a2","week":"w3-4","module":1,"question":"По Microsoft x64 calling convention первые 4 аргумента передаются через:","answers":["rdi, rsi, rdx, rcx","rcx, rdx, r8, r9","стек","rax, rbx, rcx, rdx"],"correct":1,"explain":"Microsoft x64: rcx, rdx, r8, r9. rdi/rsi — это System V (Linux)."},
  {"id":"q_a3","week":"w3-4","module":1,"question":"Что делает пары test eax, eax / jz label?","answers":["if (eax == 0) goto label","if (eax != 0) goto label","eax = 0","умножение на 0"],"correct":0,"explain":"test ставит ZF=1 если результат AND равен 0; jz переходит при ZF=1."},
  {"id":"q_a4","week":"w3-4","module":1,"question":"lea rax, [rcx + rcx*4] чаще всего означает:","answers":["загрузить адрес массива","rax = rcx * 5","rax = rcx + 4","сложение с проверкой переполнения"],"correct":1,"explain":"lea выполняет адресную арифметику: rcx + rcx*4 = rcx*5. Компиляторы любят lea вместо imul."},
  {"id":"q_a5","week":"w3-4","module":1,"question":"push rbp; mov rbp, rsp в начале функции — это:","answers":["оптимизация","пролог функции: сохранение кадра стека","вызов другой функции","очистка стека"],"correct":1,"explain":"Классический пролог: старый rbp сохраняется, rbp становится базой кадра."},
  # Модуль 1 — PE
  {"id":"q_pe1","week":"w6","module":1,"question":"PE-сигнатура \"PE\\0\\0\" в файле начинается с байт:","answers":["MZ","PE, 00 00","40 00","4D 5A 45 00"],"correct":1,"explain":"Сначала DOS header с 'MZ' (0x4D 0x5A), а e_lfanew указывает на 'PE\\0\\0'."},
  {"id":"q_pe2","week":"w6","module":1,"question":"IAT (Import Address Table) хранит:","answers":["адреса экспортируемых функций","адреса импортируемых функций после загрузчика","исходный код","секции .text"],"correct":1,"explain":"Загрузчик Windows заполняет IAT реальными адресами DLL-функций. Упаковщики её ломают — потому и нужен Scylla."},
  {"id":"q_pe3","week":"w6","module":1,"question":"Секция .text обычно имеет права:","answers":["чтение+запись","чтение+исполнение","только запись","чтение"],"correct":1,"explain":"Код исполняется и читается, но не пишется. W+X на код — признак распаковщика."},
  {"id":"q_pe4","week":"w6","module":1,"question":"Высокая энтропия секций PE чаще всего указывает на:","answers":["отладочную информацию","упаковку или шифрование","большой размер функций","старый компилятор"],"correct":1,"explain":"Сжатые/шифрованные данные почти случайны — энтропия близка к максимуму."},
  # Модуль 2 — Ghidra
  {"id":"q_g1","week":"w7","module":2,"question":"В Ghidra клавиша L на функции/переменной делает:","answers":["запускает анализ","открывает окно xrefs","переименовывает","блокирует символ"],"correct":2,"explain":"L = rename (Label). X — показать xrefs."},
  {"id":"q_g2","week":"w7","module":2,"question":"xrefs (cross-references) в дизассемблере показывают:","answers":["кто вызывает/ссылается на символ","ошибки анализа","секции файла","таблицу импортов"],"correct":0,"explain":"Xrefs — главная навигация: от строки «Wrong password» прыгаем к функции проверки."},
  {"id":"q_g3","week":"w8","module":2,"question":"F7 в x64dbg — это:","answers":["step over","step into","run до бряка","перезапуск"],"correct":1,"explain":"F7 = step into (внутрь вызова), F8 = step over, F9 = run."},
  {"id":"q_g4","week":"w8","module":2,"question":"Как в x64dbg заставить программу пойти по ветке «пароль верный» без знания пароля?","answers":["изменить ZF после cmp","удалить секцию .text","переименовать функцию","это невозможно"],"correct":0,"explain":"После cmp ставим ZF=1 (или NOP-им jz) — условный переход идёт в нужную ветку."},
  {"id":"q_g5","week":"w9","module":2,"question":"Первый инструмент, который запускают на незнакомом exe:","answers":["Ghidra","x64dbg","Detect It Easy","Frida"],"correct":2,"explain":"DIE без запуска файла отвечает: чем собрано, упаковано ли, энтропия. Только потом Ghidra."},
  {"id":"q_g6","week":"w9","module":2,"question":"capa выдаёт:","answers":["строки бинаря","capabilities с маппингом на MITRE ATT&CK","YARA-правила","дамп памяти"],"correct":1,"explain":"capa статически определяет способности бинаря (keylogging, C2...) и мапит их на ATT&CK."},
  # Модуль 3 — crackmes
  {"id":"q_cr1","week":"w10-11","module":3,"question":"Типовой паттерн проверки пароля в простом crackme:","answers":["шифрование всего exe","сравнение ввода со строкой/алгоритмом → jz/jnz → Ok/No","виртуализация кода","соль + bcrypt"],"correct":1,"explain":"Почти всегда: cmp/strcmp → условный переход на ветку «Correct». Ищем её от строк через xrefs."},
  {"id":"q_cr2","week":"w10-11","module":3,"question":"Ключген отличается от патча тем, что:","answers":["меняет байты в файле","вычисляет валидный ключ по алгоритму проверки","работает только в отладчике","удаляет проверку"],"correct":1,"explain":"Патч ломает проверку; ключген требует ПОЛНОГО понимания алгоритма — поэтому ценится выше."},
  {"id":"q_cr3","week":"w14","module":3,"question":"IsDebuggerPresent проверяет:","answers":["наличие отладчика через PEB (BeingDebugged)","файлы в системе","окружение сети","подпись файла"],"correct":0,"explain":"Читает флаг BeingDebugged в PEB. Обход: после вызова обнулить eax или пропатчить PEB."},
  {"id":"q_cr4","week":"w14","module":3,"question":"Бряк на доступ к памяти (hardware breakpoint) полезен, когда:","answers":["нужно остановиться на чтении/записи конкретного адреса","программа упакована","нет исходников","нужно пропатчить файл"],"correct":0,"explain":"Классика: не знаем, ГДЕ проверяется флаг — ставим бряк на память флага и находим код."},
  # Модуль 5 — пайплайн
  {"id":"q_ml1","week":"w18","module":5,"question":"Зачем FakeNet-NG в лабе малварь-анализа?","answers":["блокирует всю сеть","эмулирует интернет: малварь «стучится» и отвечает фейковый сервер","делает скриншоты","патчит PE"],"correct":1,"explain":"Сэмпл думает, что интернет настоящий, и сливает конфиг/C2-запросы в перехватчик."},
  {"id":"q_ml2","week":"w19-20","module":5,"question":"OEP (Original Entry Point) при распаковке — это:","answers":["точка входа упаковщика","точка входа оригинального кода после распаковки","адрес IAT","конец .text"],"correct":1,"explain":"Приём: бряк на VirtualAlloc/VirtualProtect, «run до пользовательского кода» — там OEP, дальше дамп + Scylla."},
  {"id":"q_ml3","week":"w19-20","module":5,"question":"Финальные артефакты анализа сэмпла:","answers":["скриншоты и комментарии","YARA-правило, IOC, отчёт","только дамп","ключген"],"correct":1,"explain":"Каждый анализ заканчивается воспроизводимыми артефактами: детект, индикаторы компрометации, отчёт."},
  {"id":"q_ml4","week":"w18","module":5,"question":"Железные правила работы с сэмплами:","answers":["можно запускать на хосте с антивирусом","только ВМ + снапшот, без общих папок и реальной сети","запускать через администратора на хосте","не важно, если файл маленький"],"correct":1,"explain":"Изоляция или ничего: снапшот до запуска, host-only сеть, никаких shared folders."},
  # Модуль 6 — AI
  {"id":"q_ai1","week":"w21","module":6,"question":"MCP — это:","answers":["формат PE-заголовка","протокол подключения агента к инструментам (Ghidra, x64dbg)","антивирус","компилятор"],"correct":1,"explain":"Model Context Protocol: агент (Claude Code, Cursor...) получает тула дизассемблера/отладчика и действует сам."},
  {"id":"q_ai2","week":"w21","module":6,"question":"Известное ограничение AI-реверса:","answers":["агент не умеет читать строки","агент уверенно ошибается в именах функций — крипто/C2 проверять руками","агент не работает с Ghidra","агент не понимает x86"],"correct":1,"explain":"ИИ силён в рутине (навигация, переименование), но врёт уверенно в критичных местах — нужна ручная валидация."},
  {"id":"q_ai3","week":"w22","module":6,"question":"Чему учит исследование arXiv 2605.30667?","answers":["строки в бинаре могут быть промпт-инъекцией против RE-агента","Ghidra лучше IDA","LLM нельзя использовать в security","симлинки опасны"],"correct":0,"explain":"Малварь «разговаривает» с ассистентом через строки сэмпла — изолируйте среду и секреты агента."},
  {"id":"q_ai4","week":"w22","module":6,"question":"Перед установкой чужого скилла (SKILL.md) надо:","answers":["доверять количеству звёзд","проверить через SkillSpector + прочитать код руками","дать ему доступ к сети","установить сразу в несколько агентов"],"correct":1,"explain":"Скиллы — вектор атаки: промпт-инъекции, эксфильтрация, supply-chain. NVIDIA SkillSpector сканирует их."},
]

# ---------- achievements ----------
achievements = [
    {"id":"start","name":"🧭 Первые шаги","desc":"Сдан PSet0: лаборатория собрана, журнал создан","xp":50},
    {"id":"c_master","name":"🦀 Владею C","desc":"Сданы PSet1 (password_validator) и все квизы модуля 1 по C","xp":100},
    {"id":"asm_master","name":"⚙️ Читаю ассемблер","desc":"Сданы PSet2, PSet3 — ручная декомпиляция и паттерны","xp":150},
    {"id":"pe_master","name":"📦 Разбираю PE","desc":"Написан собственный парсер PE (PSet4)","xp":150},
    {"id":"ghidra_first","name":"🐙 Ghidra не пугает","desc":"Сданы PSet5–6: бинарь причёсан, 3 crackmes решены","xp":200},
    {"id":"triage","name":"🔬 Триаж-глаз","desc":"PSet7: три отчёта триажа готовы","xp":100},
    {"id":"crackmes_20","name":"🥊 Двадцать голов","desc":"20+ crackmes решено (модуль 3)","xp":300},
    {"id":"keygen","name":"🗝️ Ключген","desc":"Написан собственный ключген с write-up","xp":250},
    {"id":"antidebug","name":"🕵️ Обошёл анти-отладку","desc":"IsDebuggerPresent и друзья побеждены (PSet12)","xp":200},
    {"id":"pwn_college","name":"🎓 Студент pwn.college","desc":"Закрыты модули Assembly и RE (PSet13–14)","xp":300},
    {"id":"lab_ready","name":"☣️ Лаборатория зла","desc":"DEFCON-лаба собрана: ВМ + FakeNet-NG (PSet / нед. 18)","xp":200},
    {"id":"full_pipeline","name":"🩸 Полный пайплайн","desc":"PSet16: сэмпл разобран от триажа до YARA + отчёт","xp":500},
    {"id":"ai_bridge","name":"🤖 Агент подключён","desc":"GhidraMCP работает, таблица доверия заполнена (PSet17)","xp":250},
    {"id":"quiz_90","name":"🧠 Скальпель","desc":"90%+ правильных ответов в тренажёре","xp":200},
    {"id":"final","name":"🏆 Выпускник RE-50","desc":"Финальный проект сдан и защищён","xp":1000},
]

data = {
    "version": 1,
    "course": {
        "title": "RE-50: Реверс-инжиниринг с нуля",
        "subtitle": "24 недели · формат CS50 · от нуля до самостоятельного анализа бинарников",
        "rules": [
            "Практика важнее чтения — 70/30.",
            "PSet сдан = решён + write-up в журнале + можешь объяснить каждое решение.",
            "Застрял 3 дня — можно подсказку, но потом решаешь с нуля заново.",
            "Декомпилятор врёт — читай ассемблер.",
            "ИИ — ускоритель понимания, а не его замена.",
            "Сэмплы запускать ТОЛЬКО в ВМ со снапшотом, без сети и общих папок."
        ],
    },
    "modules": [{"id": k, "name": v[0], "icon": v[1]} for k, v in MODULE_NAMES.items()],
    "weeks": weeks,
    "quizzes": quizzes,
    "achievements": achievements,
    "resources": resources,
}

import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT, 'w', encoding='utf-8') as f:
    json.dump(data, f, ensure_ascii=False, indent=1)

print("weeks:", len(weeks), "| quizzes:", len(quizzes), "| achievements:", len(achievements), "| resources:", len(resources))
for w in weeks:
    print(f"  {w['id']:8} m{w['module']} lec={len(w['lectures'])} lab={'Y' if w['lab'] else '-'} pset={len(w['psets'])} chk={len(w['checkpoint'])} | {w['title'][:40]}")
