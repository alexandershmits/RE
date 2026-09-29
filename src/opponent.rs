// Socratic opponent: банк каверзных вопросов и эвристическая оценка ответов.
// Режим 1 — встроенные эвристики (офлайн); режим 2 — Ollama, если запущен на localhost:11434.

pub struct OpponentQuestion {
    pub topic: &'static str,
    pub question: &'static str,
    /// ключевые слова, ожидаемые в хорошем ответе (регистронезависимо)
    pub expected: &'static [&'static str],
    /// catch-фразы, выдающие пересказ вместо понимания
    pub empty_markers: &'static [&'static str],
}

pub const QUESTIONS: &[OpponentQuestion] = &[
    OpponentQuestion {
        topic: "Ассемблер",
        question: "Чем lea eax, [rbx+4] отличается от mov eax, [rbx+4]? Когда второе упадёт, а первое — нет?",
        expected: &["адрес", "разыменов", "памят", "вычислен", "смещен"],
        empty_markers: &["просто", "не знаю", "то же самое"],
    },
    OpponentQuestion {
        topic: "Ассемблер",
        question: "Почему компилятор обнуляет регистр через xor eax, eax, а не mov eax, 0? Два независимых ответа.",
        expected: &["короче", "байт", "флаг", "zeroing", "зависим"],
        empty_markers: &["так принято", "оптимизатор так решил"],
    },
    OpponentQuestion {
        topic: "Ассемблер",
        question: "В чём разница между jg и ja? Приведи значения флагов, при которых они расходятся.",
        expected: &["знак", "sign", "sf", "of", "беззнак", "unsigned", "cf"],
        empty_markers: &["одно и то же", "похожи"],
    },
    OpponentQuestion {
        topic: "PE",
        question: "Что такое ImageBase и почему после ASLR все адреса в Ghidra съехали? Что такое релокации?",
        expected: &["reloc", "релокац", "предпочтит", "смещен", "delta", "база"],
        empty_markers: &["случайн", "баг"],
    },
    OpponentQuestion {
        topic: "PE",
        question: "Как по таблице импортов понять, что делает бинарь БЕЗ запуска? Приведи 3 конкретных признака.",
        expected: &["createfile", "virtualalloc", "loadlibrary", "winexec", "socket", "regset", "письм", "импорт"],
        empty_markers: &["посмотреть", "запустить"],
    },
    OpponentQuestion {
        topic: "Упаковка",
        question: "Как найти OEP упакованного бинаря? Опиши СВОЙ порядок действий в x64dbg, а не из книги.",
        expected: &["popad", "push", "jmp", "стек", ".section", "точку вход", "бреяк", "спуск"],
        empty_markers: &["нажать", "программа сама"],
    },
    OpponentQuestion {
        topic: "Упаковка",
        question: "Почему после дампа бинарь не запускается? Что именно чинит Scylla и почему IAT ломается?",
        expected: &["iat", "адрес", "импорт", "пересобр", "таблиц", "va", "rva"],
        empty_markers: &["не знаю", "магия"],
    },
    OpponentQuestion {
        topic: "Хеши",
        question: "Ты видишь цикл h = h*31 + c. Почему это почти наверняка проверка пароля и как её обойти БЕЗ брутфорса?",
        expected: &["обратн", "инверт", "модул", "подбор", "инверс", "реш", "уравнен"],
        empty_markers: &["брутфорс", "сложно"],
    },
    OpponentQuestion {
        topic: "Отладка",
        question: "Что такое анти-отладка через IsDebuggerPresent и как её нейтрализовать? Назови минимум 2 способа.",
        expected: &["пеб", "peb", "патч", "флаг", "платформ", "syscall", "хук", "обход", "zec", "ntp"],
        empty_markers: &["удалить", "не запускать"],
    },
    OpponentQuestion {
        topic: "Ghidra",
        question: "Ты нашёл функцию без имени sub_1400. По каким признакам ты поймёшь, что это main? По каким — что это крипто?",
        expected: &["строк", "xref", "вызов", "аргумент", "констант", "sbox", "таблиц", "цикл", "перемеш"],
        empty_markers: &["декомпилятор покажет", "повезёт"],
    },
    OpponentQuestion {
        topic: "Методология",
        question: "Твой анализ даёт вывод X, но динамический прогон противоречит. Чьим выводам верить и почему?",
        expected: &["динамик", "провер", "гипотез", "эксперимент", "лог", "трасс"],
        empty_markers: &["статик", "декомпил"],
    },
    OpponentQuestion {
        topic: "Методология",
        question: "Ты нашёл строку с «паролем» в бинаре. Почему это может быть ловушка? Как проверить?",
        expected: &["фейк", "decoy", "ловушк", "не использ", "провер", "сравнен", "реальн", "ветк"],
        empty_markers: &["ввести", "повезёт"],
    },
    OpponentQuestion {
        topic: "AI-реверс",
        question: "AI-агент переименовал функцию в check_license и ты «поверил». Какая ошибка? Как валидировать?",
        expected: &["галлюцинац", "провер", "динамик", "xref", "сравнен", "эксперимент", "вручн"],
        empty_markers: &["доверяю", "умнее меня"],
    },
    OpponentQuestion {
        topic: "AI-реверс",
        question: "Ты скормил упакованный бинарь LLM-агенту и получил правдоподобный отчёт. Что не так и почему это опасно?",
        expected: &["распаков", "псевдокод", "галлюцинац", "мусор", "провер", "полн"],
        empty_markers: &["всё ок", "удобно"],
    },
];

/// Оценка ответа: эвристика — ищем ключевые слова и ловим пустые фразы.
pub struct Verdict {
    pub score: u8, // 0..100
    pub critique: String,
}

pub fn evaluate(q: &OpponentQuestion, answer: &str) -> Verdict {
    let a = answer.to_lowercase();
    if a.trim().len() < 20 {
        return Verdict {
            score: 0,
            critique: "Слишком коротко. Оппонент ждёт обоснование, а не телеграфу.".into(),
        };
    }
    let mut hits = 0;
    for k in q.expected {
        if a.contains(k) {
            hits += 1;
        }
    }
    let mut empty = 0;
    for m in q.empty_markers {
        if a.contains(m) {
            empty += 1;
        }
    }
    let coverage = hits as f32 / q.expected.len() as f32;
    let base = (coverage * 90.0) as u8;
    let score = base.saturating_sub(empty * 25).min(100);
    let critique = if empty > 0 {
        "В ответе есть фразы-пересказ («так принято», «программа сама»). Оппонент атакует именно их: перескажи механизм, а не привычку.".to_string()
    } else if hits == 0 {
        "Ни один ключевой термин не назван. Похоже, тема не понята — вернись к лекции и попробуй снова.".to_string()
    } else if hits < q.expected.len() {
        format!("Часть механики раскрыта ({hits}/{} ключевых идей), но не вся. Что ты упустил? Подсказка: тема «{}».", q.expected.len(), q.topic)
    } else {
        "Все ключевые идеи названы. Но не расслабляйся: настоящий оппонент продолжит спрашивать «почему?» по каждому пункту.".to_string()
    };
    Verdict { score, critique }
}

/// Проверка Ollama: если сервер запущен — режим «живой LLM».
pub fn ollama_available() -> bool {
    std::net::TcpStream::connect("127.0.0.1:11434").is_ok()
}

/// Запрос к Ollama через сырой HTTP/1.1 (std-only, без reqwest).
pub fn ollama_ask(model: &str, topic: &str, student_answer: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let prompt = format!(
        "Ты — Socratic-оппонент студента по реверс-инжинирингу. Тема: {topic}. Ответ студента: {student_answer}. Задай ОДИН каверзный уточняющий вопрос, атакующий слабое место ответа. Не давай ответов. По-русски, кратко."
    );
    let body = format!(
        "{{\"model\":\"{model}\",\"prompt\":{},\"stream\":false}}",
        serde_json::to_string(&prompt).unwrap_or_else(|_| "\"?\"".into())
    );
    let mut stream = TcpStream::connect("127.0.0.1:11434")
        .map_err(|_| "Ollama не запущен (127.0.0.1:11434). Установите с ollama.com и выполните: ollama pull llama3.1".to_string())?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(120)))
        .ok();
    let req = format!(
        "POST /api/generate HTTP/1.1\r\nHost: 127.0.0.1:11434\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut resp = String::new();
    stream
        .read_to_string(&mut resp)
        .map_err(|e| e.to_string())?;
    // тело после \r\n\r\n
    let json_part = resp.split("\r\n\r\n").nth(1).unwrap_or("");
    let v: serde_json::Value = serde_json::from_str(json_part)
        .map_err(|_| format!("Неожиданный ответ Ollama: {}", &resp[..resp.len().min(200)]))?;
    Ok(v["response"]
        .as_str()
        .unwrap_or("(пустой ответ)")
        .to_string())
}
