// Socratic opponent: банк каверзных вопросов и эвристическая оценка ответов.
// Режим 1 — встроенные эвристики (офлайн); режим 2 — Ollama, если запущен на localhost:11434
// (запросы уходят только на этот адрес, наружу ничего не отправляется).

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

pub struct Verdict {
    /// 0..=100
    pub score: u8,
    pub critique: String,
}

/// Сколько слов минимум должно быть в ответе, чтобы он считался объяснением, а не списком терминов.
const MIN_WORDS: usize = 8;
/// Потолок балла за ответ-«набор ключевых слов».
const STUFFED_CAP: u32 = 35;

/// Эвристическая оценка: ключевые идеи (60% списка — полный балл), штраф за пустые фразы,
/// потолок для ответов, состоящих из перечисления терминов.
pub fn evaluate(q: &OpponentQuestion, answer: &str) -> Verdict {
    let a = answer.to_lowercase();
    if a.trim().chars().count() < 20 {
        return Verdict {
            score: 0,
            critique: "Слишком коротко. Оппонент ждёт обоснование, а не телеграфу.".into(),
        };
    }
    let words: Vec<&str> = a.split_whitespace().collect();
    let hits = q.expected.iter().filter(|k| a.contains(**k)).count();
    let empty = q.empty_markers.iter().filter(|m| a.contains(**m)).count();
    let needed = (q.expected.len() * 6).div_ceil(10).max(1);
    let mut score = (hits.min(needed) * 90 / needed) as u32;
    if hits == q.expected.len() {
        score += 10;
    }
    score = score.saturating_sub(empty as u32 * 25);
    let keyword_words = words
        .iter()
        .filter(|w| q.expected.iter().any(|k| w.contains(k)))
        .count();
    let stuffed = words.len() < MIN_WORDS || (words.len() < 25 && keyword_words * 2 > words.len());
    if stuffed {
        score = score.min(STUFFED_CAP);
    }
    let critique = if stuffed && hits > 0 {
        "Похоже на перечисление терминов. Оппонент просит связное объяснение: что происходит, в каком порядке и почему.".to_string()
    } else if empty > 0 {
        "В ответе есть фразы-пересказ («так принято», «программа сама»). Оппонент атакует именно их: перескажи механизм, а не привычку.".to_string()
    } else if hits == 0 {
        "Ни один ключевой термин не назван. Похоже, тема не понята — вернись к лекции и попробуй снова.".to_string()
    } else if hits < q.expected.len() {
        format!(
            "Часть механики раскрыта ({hits}/{} ключевых идей), но не вся. Что ты упустил? Подсказка: тема «{}».",
            q.expected.len(),
            q.topic
        )
    } else {
        "Все ключевые идеи названы. Но не расслабляйся: настоящий оппонент продолжит спрашивать «почему?» по каждому пункту.".to_string()
    };
    Verdict {
        score: score.min(100) as u8,
        critique,
    }
}

// ── Ollama (локальная LLM) ──

pub const OLLAMA_ADDR: &str = "127.0.0.1:11434";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
const REPLY_TIMEOUT: Duration = Duration::from_secs(120);

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

fn connect(timeout: Duration) -> std::io::Result<TcpStream> {
    let addr: SocketAddr = OLLAMA_ADDR.parse().expect("OLLAMA_ADDR — валидный адрес");
    TcpStream::connect_timeout(&addr, timeout)
}

/// Запущен ли Ollama. Блокирует не дольше 300 мс — вызывать из фонового потока.
pub fn ollama_available() -> bool {
    connect(CONNECT_TIMEOUT).is_ok()
}

/// Имя модели: буквы, цифры и `. _ : / -`, до 64 символов.
pub fn valid_model_name(model: &str) -> bool {
    !model.is_empty()
        && model.len() <= 64
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '-'))
}

/// HTTP/1.0-запрос: сервер отвечает без chunked-кодирования и сам закрывает соединение.
pub fn build_request(model: &str, topic: &str, student_answer: &str) -> String {
    let prompt = format!(
        "Ты — Socratic-оппонент студента по реверс-инжинирингу. Тема: {topic}. Ответ студента: {student_answer}. \
         Задай ОДИН каверзный уточняющий вопрос, атакующий слабое место ответа. Не давай ответов. По-русски, кратко."
    );
    let body = serde_json::json!({ "model": model, "prompt": prompt, "stream": false }).to_string();
    format!(
        "POST /api/generate HTTP/1.0\r\nHost: {OLLAMA_ADDR}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Разбирает сырой HTTP-ответ Ollama в текст модели.
pub fn parse_response(raw: &str) -> Result<String, String> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or("Неожиданный ответ Ollama: нет заголовков")?;
    let status: u16 = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or("Неожиданный ответ Ollama: нет статуса")?;
    let json: serde_json::Value = serde_json::from_str(body.trim()).map_err(|_| {
        format!(
            "Неожиданный ответ Ollama (HTTP {status}): {}",
            body.chars().take(200).collect::<String>()
        )
    })?;
    if let Some(err) = json["error"].as_str() {
        return Err(format!("Ollama: {err}"));
    }
    if status != 200 {
        return Err(format!("Ollama вернул HTTP {status}"));
    }
    match json["response"].as_str().map(str::trim) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => Err("Ollama вернул пустой ответ".into()),
    }
}

/// Уточняющий вопрос от локальной модели. Блокирует до 2 минут — только из фонового потока.
pub fn ollama_ask(model: &str, topic: &str, student_answer: &str) -> Result<String, String> {
    if !valid_model_name(model) {
        return Err("Недопустимое имя модели".into());
    }
    let mut stream = connect(Duration::from_secs(2)).map_err(|_| {
        format!("Ollama не запущен ({OLLAMA_ADDR}). Установите с ollama.com и выполните: ollama pull llama3.1")
    })?;
    stream
        .set_read_timeout(Some(REPLY_TIMEOUT))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(build_request(model, topic, student_answer).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    parse_response(&String::from_utf8_lossy(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEA: &OpponentQuestion = &QUESTIONS[0];

    const GOOD: &str = "lea вычисляет адрес и кладёт его в регистр, не обращаясь к памяти, а mov разыменовывает адрес и читает память; mov упадёт, если указатель невалиден";

    #[test]
    fn short_answer_scores_zero() {
        assert_eq!(evaluate(LEA, "не знаю").score, 0);
    }

    #[test]
    fn real_explanation_scores_well() {
        let v = evaluate(LEA, GOOD);
        assert!(v.score >= 50, "score={}", v.score);
    }

    #[test]
    fn filler_phrases_are_penalised() {
        let bad = evaluate(LEA, "это просто одно и то же, наверное, не знаю точно но думаю что просто одинаково работают всегда");
        assert!(bad.score < evaluate(LEA, GOOD).score);
        assert!(bad.critique.contains("пересказ"));
    }

    #[test]
    fn keyword_stuffing_does_not_beat_an_explanation() {
        let stuffed = evaluate(LEA, "lea адрес разыменов памят вычислен смещен");
        assert!(
            stuffed.score <= STUFFED_CAP as u8,
            "score={}",
            stuffed.score
        );
        assert!(stuffed.score < evaluate(LEA, GOOD).score);
        assert!(stuffed.critique.contains("перечисление"));
    }

    #[test]
    fn every_question_is_answerable_by_its_own_keywords() {
        for q in QUESTIONS {
            assert!(
                q.expected.len() >= 4 && !q.empty_markers.is_empty(),
                "{}",
                q.question
            );
            let sentence = format!(
                "Я объясняю так: {} — вот как это устроено на самом деле у меня",
                q.expected.join(" и потом ")
            );
            assert!(evaluate(q, &sentence).score >= 90, "{}", q.question);
        }
        assert_eq!(QUESTIONS.len(), 14);
    }

    #[test]
    fn request_is_valid_http10_with_escaped_json() {
        let req = build_request("llama3.1", "PE", "он сказал \"привет\"\nи ушёл");
        assert!(req.starts_with("POST /api/generate HTTP/1.0\r\n"));
        let (head, body) = req.split_once("\r\n\r\n").unwrap();
        let json: serde_json::Value = serde_json::from_str(body).expect("тело — валидный JSON");
        assert_eq!(json["model"], "llama3.1");
        assert_eq!(json["stream"], false);
        assert!(json["prompt"].as_str().unwrap().contains("\"привет\""));
        assert!(
            head.contains(&format!("Content-Length: {}", body.len())),
            "длина в байтах, не в символах"
        );
    }

    #[test]
    fn model_names_are_validated() {
        for ok in ["llama3.1", "qwen2.5:7b", "library/mistral", "phi-3_mini"] {
            assert!(valid_model_name(ok), "{ok}");
        }
        for bad in ["", "a\"b", "x y", "модель", &"m".repeat(65)] {
            assert!(!valid_model_name(bad), "{bad}");
        }
    }

    #[test]
    fn responses_are_parsed_and_errors_surface() {
        let ok = "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"response\": \" Почему? \"}";
        assert_eq!(parse_response(ok).unwrap(), "Почему?");
        let missing = "HTTP/1.0 404 Not Found\r\n\r\n{\"error\":\"model 'x' not found\"}";
        assert!(parse_response(missing)
            .unwrap_err()
            .contains("model 'x' not found"));
        assert!(parse_response("HTTP/1.0 500 X\r\n\r\n{\"foo\":1}")
            .unwrap_err()
            .contains("500"));
        assert!(
            parse_response("HTTP/1.0 200 OK\r\n\r\n{\"response\":\"  \"}")
                .unwrap_err()
                .contains("пустой")
        );
        assert!(parse_response("HTTP/1.0 200 OK\r\n\r\n<html>")
            .unwrap_err()
            .contains("Неожиданный"));
        assert!(parse_response("мусор").is_err());
    }
}
