// Интерактивные микро-симуляторы для закрепления теории.

use crate::emulator::Machine;
use crate::rng::Rng;
use std::sync::OnceLock;

pub struct RegTask {
    pub title: String,
    pub code: Vec<String>,
    /// Регистры, значения которых нужно назвать, и их начальные значения.
    pub init: Vec<(&'static str, u64)>,
    /// Эталон: регистр -> значение после `code`. Считает эмулятор, а не человек.
    pub answers: Vec<(&'static str, u64)>,
    pub explain: String,
}

impl RegTask {
    fn build(title: &str, code: &[&str], init: &[(&'static str, u64)], explain: &str) -> RegTask {
        let code: Vec<String> = code.iter().map(|s| s.to_string()).collect();
        let mut m = Machine::new();
        for (reg, value) in init {
            m.set(reg, *value).expect("допустимый регистр");
        }
        m.run(&code)
            .unwrap_or_else(|e| panic!("задача «{title}»: {e}"));
        let answers = init
            .iter()
            .map(|(reg, _)| (*reg, m.get(reg).expect("допустимый регистр")))
            .collect();
        RegTask {
            title: title.into(),
            code,
            init: init.to_vec(),
            answers,
            explain: explain.into(),
        }
    }

    pub fn all() -> &'static [RegTask] {
        static TASKS: OnceLock<Vec<RegTask>> = OnceLock::new();
        TASKS.get_or_init(|| {
            vec![
                RegTask::build(
                    "1. mov и lea — базовая арифметика",
                    &["mov rax, 0x10", "mov rbx, 0x4", "lea rcx, [rax + rbx*2]", "add rax, rbx"],
                    &[("rax", 0), ("rbx", 0), ("rcx", 0)],
                    "lea rcx,[rax+rbx*2] = 0x10+0x4*2 = 0x18 (lea НЕ читает память, только арифметика). add: 0x10+0x4 = 0x14.",
                ),
                RegTask::build(
                    "2. 32-битные обнуления старших бит",
                    &["mov rax, 0xFFFFFFFFFFFFFFFF", "mov eax, 0x1"],
                    &[("rax", 0)],
                    "Запись в 32-битный регистр (eax) ОБНУЛЯЕТ старшие 32 бита rax. Это не 0x100000001 — именно 0x1!",
                ),
                RegTask::build(
                    "3. Стек: push и pop",
                    &["mov rax, 0xAAAA", "mov rbx, 0xBBBB", "push rax", "push rbx", "pop rcx", "pop rdx"],
                    &[("rcx", 0), ("rdx", 0)],
                    "Стек LIFO: последним положили rbx — первым забрали в rcx (0xBBBB), затем rdx = 0xAAAA.",
                ),
                RegTask::build(
                    "4. cmp/test + флаги",
                    &["mov eax, 0x5", "cmp eax, 0x5", "jz done", "mov eax, 0xFF", "done:"],
                    &[("eax", 0)],
                    "cmp 5,5 ставит ZF=1 → jz срабатывает → mov eax,0xFF пропущен. Классический паттерн проверки пароля!",
                ),
                RegTask::build(
                    "5. xor и сдвиги (расшифровка)",
                    &["mov rax, 0xFF", "xor rbx, rbx", "mov rcx, 0x3", "shl rax, cl", "inc rax"],
                    &[("rax", 0), ("rbx", 0x77), ("rcx", 0)],
                    "xor rbx,rbx = 0 (обнуление). shl 0xFF на 3 = 0x7F8, inc → 0x7F9. Сдвиг на cl использует младшие 6 бит rcx; сам rcx остаётся 3.",
                ),
            ]
        })
    }
}

pub struct PeTask {
    pub title: String,
    pub bytes: String, // hex dump pre-formatted
    pub question: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

impl PeTask {
    pub fn all() -> &'static [PeTask] {
        static TASKS: OnceLock<Vec<PeTask>> = OnceLock::new();
        TASKS.get_or_init(|| vec![
            PeTask {
                title: "1. DOS header".into(),
                bytes: "00000000  4D 5A 90 00 03 00 00 00  04 00 00 00 FF FF 00 00\n00000010  B8 00 00 00 00 00 00 00  40 00 00 00 00 00 00 00\n00000020  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00\n00000030  00 00 00 00 00 00 00 00  00 00 00 00 E8 00 00 00\n0000003C  E8 00 00 00  <- e_lfanew".into(),
                question: "Что здесь означает байтовая пара 4D 5A в начале файла?".to_string(),
                answers: vec!["Магия 'MZ' — DOS-заголовок, признак PE".to_string(), "Размер секции .text".to_string(), "Адрес точки входа".to_string(), "Это повреждённый файл".to_string()],
                correct: 0,
                explain: "'MZ' = Mark Zbikowski, разработчик MS-DOS. Это сигнатура начала PE. e_lfanew (смещение 0x3C) = 0xE8 — там лежит 'PE\\0\\0'.".into(),
            },
            PeTask {
                title: "2. Number of Sections".into(),
                bytes: "000000E8  50 45 00 00  <- PE signature\n000000EC  4C 01        <- Machine (i386)\n000000EE  06 00        <- NumberOfSections\n000000F0  3A 8B 2F 68  <- TimeDateStamp\n000000F4  00 00 00 00  <- PointerToSymbolTable\n000000F8  00 00 00 00\n000000FC  F0 00        <- SizeOfOptionalHeader\n000000FE  22 00        <- Characteristics".into(),
                question: "Сколько секций в этом PE-файле?".to_string(),
                answers: vec!["4".to_string(), "6".to_string(), "22".to_string(), "0x1C4".to_string()],
                correct: 1,
                explain: "NumberOfSections = 0x0006 = 6 секций (little-endian!). Обратите внимание на порядок байтов: 06 00, не 00 06.".into(),
            },
            PeTask {
                title: "3. Секция с кодом".into(),
                bytes: ".text  VirtualSize:  0x00004C20\n.text  VirtualAddr: 0x00001000\n.text  SizeOfRaw:   0x00004E00\n.text  Chars:       0x60000020\n\n.data  VirtualSize:  0x00001240\n.data  VirtualAddr: 0x00006000\n.data  SizeOfRaw:   0x00000A00\n.data  Chars:       0xC0000040".into(),
                question: "Какие права у секции .text? (расшифровка Characteristics: 0x20=CODE, 0x40=INIT_DATA, 0x20000000=EXECUTE, 0x40000000=READ, 0x80000000=WRITE)".to_string(),
                answers: vec!["Read + Write (данные)".to_string(), "Code + Execute + Read".to_string(), "Code + Write".to_string(), "Только Read".to_string()],
                correct: 1,
                explain: "0x60000020 = 0x20 (CODE) + 0x20000000 (EXECUTE) + 0x40000000 (READ). У .data (0xC0000040) — RW+инициализированные данные. W+X у секции кода = красный флаг упаковщика.".into(),
            },
            PeTask {
                title: "4. RVA → VA".into(),
                bytes: "Optional Header:\nImageBase:            0x00400000\nAddressOfEntryPoint:  0x00001234  (RVA)\n\nSection .text:\nVirtualAddress:  0x00001000 (RVA)\nPointerToRawData: 0x00000400 (file offset)".into(),
                question: "Чему равен ВИРТУАЛЬНЫЙ адрес (VA) точки входа?".to_string(),
                answers: vec!["0x401234".to_string(), "0x411234".to_string(), "0x123400".to_string(), "0x400400".to_string()],
                correct: 0,
                explain: "VA = ImageBase + RVA = 0x400000 + 0x1234 = 0x401234. А PointerToRawData — это file offset, третье адресное пространство, в VA не складывается.".into(),
            },
            PeTask {
                title: "5. Подозрительная секция".into(),
                bytes: ".text  Chars: 0x60000020  (CODE, EXEC, READ)\n UPX0  Chars: 0xC0000040  (RW)\n UPX1  Chars: 0xE0000040  (RWE!)\n UPX2  Chars: 0xC0000040  (RW)\nEntry: RVA 0x00091000 (внутри UPX1)\nImports: 2 DLL, 5 функций всего".into(),
                question: "Что перед вами?".to_string(),
                answers: vec!["Обычная программа на Go".to_string(), "Упаковщик UPX: entry в секции с RWE, почти нет импортов".to_string(), "Драйвер ядра".to_string(), "Библиотека .NET".to_string()],
                correct: 1,
                explain: "Триада упаковщика: секции UPX*, точка входа в нестандартной RWE-секции, минимальный IAT (настоящие импорты появятся после распаковки).".into(),
            },
        ])
    }
}

pub struct OepTask {
    pub trace: Vec<String>,
    pub question: String,
    pub answers: Vec<String>,
    pub correct: usize,
    pub explain: String,
}

impl OepTask {
    pub fn all() -> &'static [OepTask] {
        static TASKS: OnceLock<Vec<OepTask>> = OnceLock::new();
        TASKS.get_or_init(|| vec![
            OepTask {
                trace: vec!["EP     0x00401000  pushad                      ; упаковщик начал работу".to_string(), "0x00401001  call    $+5".to_string(), "0x00401010  mov     ebp, esp".to_string(), "0x00401500  call    VirtualAlloc (выделить буфер)".to_string(), "0x00401580  loop    unpack_cycle   ; расшифровка секций".to_string(), "0x00401600  call    VirtualFree".to_string(), "0x00401610  call    LoadLibrary / GetProcAddress  ; восстановление IAT".to_string(), "0x00401620  popad".to_string(), "0x00401621  jmp     0x00452ABC   ; <- последний прыжок".to_string(), "0x00452ABC  push    ebp          ; <-- ЗДЕСЬ".to_string(), "0x00452ABD  mov     ebp, esp".to_string(), "0x00452ABF  sub     esp, 0x400".to_string(), "0x00452AC5  call    GetVersion".to_string()],
                question: "Где OEP (Original Entry Point) оригинальной программы?".to_string(),
                answers: vec!["0x00401000 — первая инструкция".to_string(), "0x00401500 — VirtualAlloc".to_string(), "0x00452ABC — код после последнего прыжка упаковщика: push ebp; mov ebp,esp".to_string(), "0x00401620 — popad".to_string()],
                correct: 2,
                explain: "OEP — первая инструкция ПОСЛЕ того, как упаковщик закончил: epilogue (popad) → большой jmp в другую секцию → там классический пролог push ebp; mov ebp,esp. Дампиться надо на 0x00452ABC.".into(),
            },
            OepTask {
                trace: vec!["EP     0x00451000  pushad".to_string(), "0x00451050  xor     eax, eax    ; SEH-ловушка настроена".to_string(), "0x00451060  div     ecx         ; вызов исключения (анти-отладка!)".to_string(), "0x0045A000  -> SEH handler:  unpack continues".to_string(), "0x0045A100  loop    decode".to_string(), "0x0045A200  cmp     dword [esp+0x1C], 0x00451000  ; проверка контекста".to_string(), "0x0045A210  popad".to_string(), "0x0045A211  jmp     0x00401A50".to_string(), "0x00401A50  call    ds:GetCommandLineA".to_string(), "0x00401A56  push    0x64".to_string(), "0x00401A58  call    0x00401B00".to_string()],
                question: "Упаковщик использует анти-отладку через исключение (div by zero → SEH). Что это значит для поиска OEP?".to_string(),
                answers: vec!["OEP = 0x00451060, место исключения".to_string(), "Программа всегда упадёт, OEP не существует".to_string(), "Отладчик прыгнет в SEH-обработчик не так, как реальная ЦП — нельзя верить трассе до прохода через исключение; OEP — код после финального jmp (0x00401A50)".to_string(), "OEP = адрес SEH handler 0x0045A000".to_string()],
                correct: 2,
                explain: "Разница: под отладчиком исключение перехватит ОТЛАДЧИК (если не настроить pass exception to app), и трасса пойдёт не туда. OEP всё равно — после popad/jmp: здесь GetCommandLineA — импорты уже работают, значит распаковка завершена.".into(),
            },
            OepTask {
                trace: vec!["EP     0x400010  xor  ecx, ecx".to_string(), "0x400020  mov  edx, 0x401000  ; указатель на сжатые данные".to_string(), "0x400030  call aPLib_decompress".to_string(), "0x400035  mov  eax, 0x405000  ; куда распаковали".to_string(), "0x400040  jmp eax             ; переход по регистру!".to_string(), "0x405000  push 0x406000".to_string(), "0x405005  call 0x405010".to_string(), "0x405010  cmp  dword [esp+4], 0xDEADBEEF".to_string(), "0x405018  jne  self_delete".to_string()],
                question: "Чем этот упаковщик хитрее предыдущих?".to_string(),
                answers: vec!["Ничем особенным".to_string(), "Финальный переход — jmp eax (косвенный): бряк по фиксированному адресу не сработает, надо брякать на jmp eax и смотреть значение регистра".to_string(), "Он не распаковывает код".to_string(), "Здесь нет OEP".to_string()],
                correct: 1,
                explain: "Косвенный переход jmp eax — типовой приём против статического поиска финального jmp. Приёмы ловли: hardware breakpoint на EXECUTE памяти распакованной секции, либо бряк на jmp eax и чтение eax.".into(),
            },
        ])
    }
}

// ============ ГЕНЕРАТИВНЫЕ СИМУЛЯТОРЫ: бесконечная практика ============

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenKind {
    /// адрес = RIP следующей инструкции + смещение
    RipRelative,
    /// собрать значение из байтов в памяти
    LittleEndian,
    /// значение AL после серии mov/xor/add
    DecodeMov,
}

impl GenKind {
    pub const ALL: [GenKind; 3] = [
        GenKind::RipRelative,
        GenKind::LittleEndian,
        GenKind::DecodeMov,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            GenKind::RipRelative => "🎲 RIP-relative адрес",
            GenKind::LittleEndian => "🎲 Little-endian разбор",
            GenKind::DecodeMov => "🎲 Трасса регистров",
        }
    }
}

pub struct GenTask {
    pub question: String,
    /// Эталон hex-строкой без `0x`, строчными буквами (для показа).
    pub answer: String,
    /// Эталон числом: сравнение идёт по значению, а не по написанию.
    pub value: u64,
    pub explain: String,
}

/// Задача полностью определяется `(kind, seed)`.
pub fn generate(kind: &GenKind, seed: u64) -> GenTask {
    let mut r = Rng::new(seed);
    let (question, value, explain) = match kind {
        GenKind::RipRelative => {
            let rip = 0x1_4000_1000u64 + r.below(0x1000);
            let disp = r.below(0x200) as i64 - 0x100;
            let target = rip.wrapping_add_signed(disp);
            let shown = if disp < 0 {
                format!("- {:#x}", -disp)
            } else {
                format!("+ {disp:#x}")
            };
            (
                format!(
                    "Инструкция: mov rax, [rip {shown}]\nRIP на СЛЕДУЮЩЕЙ инструкции = {rip:#x}\nКакой адрес читается?"
                ),
                target,
                format!(
                    "RIP-relative: цель = RIP_следующей {} {:#x} = {target:#x}. RIP всегда указывает на СЛЕДУЮЩУЮ инструкцию, а не текущую!",
                    if disp < 0 { "−" } else { "+" },
                    disp.unsigned_abs()
                ),
            )
        }
        GenKind::LittleEndian => {
            let val = r.below(1 << 32);
            let b = (val as u32).to_le_bytes();
            let tail = (r.below(0x10000) as u16).to_le_bytes();
            (
                format!(
                    "В памяти по адресу X лежат байты (слева направо): {:02X} {:02X} {:02X} {:02X} | {:02X} {:02X}\nКакое 32-битное значение прочитает mov eax, [X]?",
                    b[0], b[1], b[2], b[3], tail[0], tail[1]
                ),
                val,
                format!(
                    "Little-endian: младший байт лежит по младшему адресу. {val:#x} хранится как {:02X} {:02X} {:02X} {:02X}, поэтому читаем байты справа налево. Последние два байта — соседняя переменная, в ответ не входят.",
                    b[0], b[1], b[2], b[3]
                ),
            )
        }
        GenKind::DecodeMov => {
            let a = r.below(0x100);
            let k = r.below(0xFF) | 1;
            let x = a ^ k;
            let sum = x + 0x10;
            let al = sum & 0xFF;
            let carry = if sum > 0xFF {
                format!(" AL — 8-битный регистр: {sum:#x} не помещается, перенос теряется, остаётся {al:#x}.")
            } else {
                String::new()
            };
            (
                format!("mov al, {a:#x}\nxor al, {k:#x}\nadd al, 0x10\nЧему равен AL (hex)?"),
                al,
                format!("{a:#x} ^ {k:#x} = {x:#x}; {x:#x} + 0x10 = {sum:#x}.{carry} Трасса пошагово: не в голове, а на бумаге!"),
            )
        }
    };
    GenTask {
        question,
        answer: format!("{value:x}"),
        value,
        explain,
    }
}

/// Шестнадцатеричное число с необязательным `0x` и пробелами по краям; регистр и ведущие нули не важны.
pub fn parse_hex(input: &str) -> Option<u64> {
    let t = input.trim();
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    if t.is_empty() {
        return None;
    }
    u64::from_str_radix(t, 16).ok()
}

/// Ответ студента верен, если совпадает значение.
pub fn check_gen(task: &GenTask, student: &str) -> bool {
    parse_hex(student) == Some(task.value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn reg_answers(i: usize) -> Vec<(&'static str, u64)> {
        RegTask::all()[i].answers.clone()
    }

    #[test]
    fn dos_header_points_at_the_pe_signature_shown_in_the_next_task() {
        let tasks = PeTask::all();
        let line = |text: &'static str, needle: &str| {
            text.lines()
                .find(|l| l.contains(needle))
                .unwrap_or_default()
        };
        let hex = |l: &str, skip| -> Vec<u8> {
            l.split_whitespace()
                .skip(skip)
                .take_while(|t| t.len() == 2 && t.chars().all(|c| c.is_ascii_hexdigit()))
                .map(|t| u8::from_str_radix(t, 16).unwrap())
                .collect()
        };
        let e_lfanew = hex(line(&tasks[0].bytes, "<- e_lfanew"), 1);
        assert_eq!(e_lfanew.len(), 4);
        let row_0x30 = hex(line(&tasks[0].bytes, "00000030"), 1);
        assert_eq!(
            row_0x30[12..],
            e_lfanew[..],
            "поле в дампе и подпись должны совпасть"
        );
        let value = u32::from_le_bytes(e_lfanew.try_into().unwrap());
        let pe_offset =
            u32::from_str_radix(tasks[1].bytes.split_whitespace().next().unwrap(), 16).unwrap();
        assert_eq!(
            value, pe_offset,
            "e_lfanew должен указывать на «PE\\0\\0» из следующей задачи"
        );
        assert!(tasks[0].explain.contains(&format!("0x{value:X}")));
    }

    #[test]
    fn register_answers_are_computed_by_the_emulator() {
        assert_eq!(reg_answers(0), [("rax", 0x14), ("rbx", 4), ("rcx", 0x18)]);
        assert_eq!(reg_answers(1), [("rax", 1)]);
        assert_eq!(reg_answers(2), [("rcx", 0xBBBB), ("rdx", 0xAAAA)]);
        assert_eq!(reg_answers(3), [("eax", 5)]);
        // регресс: в ключе задачи №5 стояли значения задачи №1 (0x14/0x4/0x18)
        assert_eq!(reg_answers(4), [("rax", 0x7F9), ("rbx", 0), ("rcx", 3)]);
        assert!(RegTask::all()
            .iter()
            .all(|t| !t.explain.is_empty() && !t.code.is_empty()));
    }

    #[test]
    fn explanations_match_the_computed_answers() {
        let t5 = &RegTask::all()[4];
        assert!(t5.explain.contains("0x7F8") && t5.explain.contains("0x7F9"));
    }

    #[test]
    fn choice_tasks_are_well_formed() {
        for t in PeTask::all() {
            assert!(t.answers.len() == 4 && t.correct < 4, "{}", t.title);
        }
        for (i, t) in OepTask::all().iter().enumerate() {
            assert!(t.answers.len() == 4 && t.correct < 4, "oep{i}");
        }
        assert_eq!((PeTask::all().len(), OepTask::all().len()), (5, 3));
    }

    #[test]
    fn same_seed_same_task_and_neighbours_differ() {
        for kind in GenKind::ALL {
            assert_eq!(generate(&kind, 42).question, generate(&kind, 42).question);
        }
        // регресс: `seed | 1` склеивал соседние сиды — из 20 задач получалось 10
        let distinct: HashSet<String> = (0..20)
            .map(|s| generate(&GenKind::RipRelative, s).question)
            .collect();
        assert_eq!(distinct.len(), 20);
    }

    #[test]
    fn rip_relative_shows_negative_displacement_as_subtraction() {
        let mut seen = (false, false);
        for seed in 0..200 {
            let t = generate(&GenKind::RipRelative, seed);
            assert!(!t.question.contains("ffffffff"), "{}", t.question);
            seen.0 |= t.question.contains("[rip - 0x");
            seen.1 |= t.question.contains("[rip + 0x");
            assert!(check_gen(&t, &t.answer));
        }
        assert_eq!(seen, (true, true));
    }

    #[test]
    fn rip_relative_answer_is_really_rip_plus_disp() {
        for seed in 0..300 {
            let t = generate(&GenKind::RipRelative, seed);
            let mut lines = t.question.lines();
            let first = lines.next().unwrap();
            let sign: i128 = if first.contains("[rip -") { -1 } else { 1 };
            let disp =
                u128::from_str_radix(first.rsplit("0x").next().unwrap().trim_end_matches(']'), 16)
                    .unwrap() as i128;
            let rip_line = lines.next().unwrap();
            let rip =
                u128::from_str_radix(rip_line.rsplit("0x").next().unwrap(), 16).unwrap() as i128;
            assert_eq!(
                i128::from(t.value),
                rip + sign * disp,
                "seed {seed}: {}",
                t.question
            );
        }
    }

    #[test]
    fn answers_are_compared_by_value_not_spelling() {
        let t = generate(&GenKind::LittleEndian, 2);
        let v = t.value;
        let variants = [
            format!("{v:x}"),
            format!("{v:X}"),
            format!("0x{v:x}"),
            format!("0X{v:X}"),
            format!("  0x{v:08X} "),
            format!("{v:016x}"),
        ];
        for ok in &variants {
            assert!(check_gen(&t, ok), "{ok}");
        }
        for bad in ["", "0x", "zzz", "-1", "0x 12"] {
            assert!(!check_gen(&t, bad), "{bad}");
        }
        assert!(!check_gen(&t, &format!("{:x}", v + 1)));
        assert!(!check_gen(&t, &format!("{v:x} 0")));
        // младшая цифра 0 в записи «0c» — ровно то, что раньше отвергалось
        let d = GenTask {
            question: String::new(),
            answer: "c".into(),
            value: 0x0c,
            explain: String::new(),
        };
        assert!(check_gen(&d, "0c") && check_gen(&d, "0x0C") && check_gen(&d, "c"));
    }

    #[test]
    fn eight_bit_overflow_is_explained_only_when_it_happens() {
        let (mut with_carry, mut without) = (None, None);
        for seed in 0..200 {
            let t = generate(&GenKind::DecodeMov, seed);
            if t.explain.contains("перенос") {
                with_carry.get_or_insert(t);
            } else {
                without.get_or_insert(t);
            }
        }
        let t = with_carry.expect("нужны задачи с переносом");
        assert!(
            t.explain.contains("не помещается") && t.value <= 0xFF,
            "{}",
            t.explain
        );
        assert!(without.is_some());
    }

    #[test]
    fn decode_mov_matches_an_independent_computation() {
        for seed in 0..500 {
            let t = generate(&GenKind::DecodeMov, seed);
            let nums: Vec<u64> = t
                .question
                .lines()
                .take(3)
                .map(|l| u64::from_str_radix(l.rsplit("0x").next().unwrap(), 16).unwrap())
                .collect();
            let expected = ((nums[0] ^ nums[1]) as u8).wrapping_add(nums[2] as u8);
            assert_eq!(t.value, u64::from(expected), "seed {seed}: {}", t.question);
        }
    }
}
