// Интерактивные микро-симуляторы для закрепления теории.

pub struct RegTask {
    pub title: String,
    pub code: Vec<String>,
    /// начальные значения регистров (hex-строки без 0x)
    pub init: Vec<(&'static str, u64)>,
    /// ответ: регистр -> итоговое значение
    pub answers: Vec<(&'static str, u64)>,
    pub explain: String,
}

impl RegTask {
    pub fn all() -> Vec<RegTask> {
        vec![
            RegTask {
                title: "1. mov и lea — базовая арифметика".into(),
                code: vec![
                    "mov rax, 0x10".into(),
                    "mov rbx, 0x4".into(),
                    "lea rcx, [rax + rbx*2]".into(),
                    "add rax, rbx".into(),
                ],
                init: vec![("rax", 0), ("rbx", 0), ("rcx", 0)],
                answers: vec![("rax", 0x14), ("rbx", 0x4), ("rcx", 0x18)],
                explain: "lea rcx,[rax+rbx*2] = 0x10+0x4*2 = 0x18 (lea НЕ читает память, только арифметика). add: 0x10+0x4 = 0x14.".into(),
            },
            RegTask {
                title: "2. 32-битные обнуления старших бит".into(),
                code: vec![
                    "mov rax, 0xFFFFFFFFFFFFFFFF".into(),
                    "mov eax, 0x1".into(),
                ],
                init: vec![("rax", 0)],
                answers: vec![("rax", 0x1)],
                explain: "Запись в 32-битный регистр (eax) ОБНУЛЯЕТ старшие 32 бита rax. Это не 0x100000001 — именно 0x1!".into(),
            },
            RegTask {
                title: "3. Стек: push и pop".into(),
                code: vec![
                    "mov rax, 0xAAAA".into(),
                    "mov rbx, 0xBBBB".into(),
                    "push rax".into(),
                    "push rbx".into(),
                    "pop rcx".into(),
                    "pop rdx".into(),
                ],
                init: vec![("rcx", 0), ("rdx", 0)],
                answers: vec![("rcx", 0xBBBB), ("rdx", 0xAAAA)],
                explain: "Стек LIFO: последним положили rbx — первым забрали в rcx (0xBBBB), затем rdx = 0xAAAA.".into(),
            },
            RegTask {
                title: "4. cmp/test + флаги".into(),
                code: vec![
                    "mov eax, 0x5".into(),
                    "cmp eax, 0x5".into(),
                    "; ZF=1, здесь стоит jz done".into(),
                    "mov eax, 0xFF".into(),
                    "done:".into(),
                ],
                init: vec![("eax", 0)],
                answers: vec![("eax", 0x5)],
                explain: "cmp 5,5 ставит ZF=1 → jz срабатывает → mov eax,0xFF пропущен. Классический паттерн проверки пароля!".into(),
            },
            RegTask {
                title: "5. xor и сдвиги (расшифровка)".into(),
                code: vec![
                    "mov rax, 0xFF".into(),
                    "xor rbx, rbx".into(),
                    "mov rcx, 0x3".into(),
                    "shl rax, cl".into(),
                    "inc rax".into(),
                ],
                init: vec![("rax", 0), ("rbx", 0x77), ("rcx", 0)],
                answers: vec![("rax", 0x14), ("rbx", 0x4), ("rcx", 0x18)],
                explain: "xor rbx,rbx = 0 (обнуление). shl 0xFF на 3 = 0x7F0, +1 = 0x7F8. Сдвиг на cl — младшие 6 бит rcx.".into(),
            },
        ]
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
    pub fn all() -> Vec<PeTask> {
        vec![
            PeTask {
                title: "1. DOS header".into(),
                bytes: "00000000  4D 5A 90 00 03 00 00 00  04 00 00 00 FF FF 00 00\n00000010  B8 00 00 00 00 00 00 00  40 00 00 00 00 00 00 00\n00000020  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00\n00000030  00 00 00 00 00 00 00 00  00 00 00 00 F0 00 00 00\n0000003C  E8 04 00 00  <- e_lfanew".into(),
                question: "Что здесь означает байтовая пара 4D 5A в начале файла?".to_string(),
                answers: vec!["Магия 'MZ' — DOS-заголовок, признак PE".to_string(), "Размер секции .text".to_string(), "Адрес точки входа".to_string(), "Это повреждённый файл".to_string()],
                correct: 0,
                explain: "'MZ' = Mark Zbikowski, разработчик MS-DOS. Это сигнатура начала PE. e_lfanew (смещение 0x3C) = 0x4E8 — там лежит 'PE\\0\\0'.".into(),
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
        ]
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
    pub fn all() -> Vec<OepTask> {
        vec![
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
        ]
    }
}


// ============ ГЕНЕРАТИВНЫЕ СИМУЛЯТОРЫ: бесконечная практика ============

/// LCG для воспроизводимой рандомизации
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self { Rng(seed | 1) }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    pub fn below(&mut self, n: u64) -> u64 { self.next() % n }
}

pub enum GenKind {
    RipRelative,   // адрес = RIP + disp
    LittleEndian,  // собери значение из байтов
    DecodeMov,     // найди значение регистра после серии mov/xor/add
}

impl GenKind {
    pub fn title(&self) -> &'static str {
        match self {
            GenKind::RipRelative => "🎲 RIP-relative адрес",
            GenKind::LittleEndian => "🎲 Little-endian разбор",
            GenKind::DecodeMov => "🎲 Трасса регистров",
        }
    }
}

pub struct GenTask {
    #[allow(dead_code)]
    pub kind_title: String,
    pub question: String,
    pub answer: String,   // hex-строка без 0x, lowercase
    pub explain: String,
}

pub fn generate(kind: &GenKind, seed: u64) -> GenTask {
    let mut r = Rng::new(seed);
    match kind {
        GenKind::RipRelative => {
            let rip = 0x140001000u64 + r.below(0x1000);
            let disp = r.below(0x200) as i64 - 0x100;
            let target = (rip as i64 + disp) as u64;
            GenTask {
                kind_title: kind.title().into(),
                question: format!(
                    "Инструкция: mov rax, [rip + {:#x}]\nRIP на СЛЕДУЮЩЕЙ инструкции = {:#x}\nКакой адрес читается?",
                    disp, rip
                ),
                answer: format!("{:x}", target),
                explain: format!(
                    "RIP-relative: цель = RIP_следующей + disp = {:#x} {:+#x} = {:#x}. RIP всегда указывает на СЛЕДУЮЩУЮ инструкцию, а не текущую!",
                    rip, disp, target
                ),
            }
        }
        GenKind::LittleEndian => {
            let val: u64 = r.below(0x100000000);
            let b = [(val & 0xFF) as u8, ((val >> 8) & 0xFF) as u8, ((val >> 16) & 0xFF) as u8, ((val >> 24) & 0xFF) as u8];
            let val4 = r.below(0x10000) as u16;
            let w = [(val4 & 0xFF) as u8, (val4 >> 8) as u8];
            GenTask {
                kind_title: kind.title().into(),
                question: format!(
                    "В памяти по адресу X лежат байты (слева направо): {:02X} {:02X} {:02X} {:02X} | {:02X} {:02X}\nКакое 32-битное значение прочитает mov eax, [X]?",
                    b[0], b[1], b[2], b[3], w[0], w[1]
                ),
                answer: format!("{:x}", val),
                explain: format!(
                    "Little-endian: младший байт по младшему адресу. {:#x} = {:02X} {:02X} {:02X} {:02X} в памяти. Первые 4 байта читаются как dword: {:#x}.",
                    val, b[0], b[1], b[2], b[3], val
                ),
            }
        }
        GenKind::DecodeMov => {
            let a = r.below(0xFF);
            let k = r.below(0xFF) | 1;
            let b = (a ^ k) & 0xFF;
            let c = (b.wrapping_add(0x10)) & 0xFF;
            GenTask {
                kind_title: kind.title().into(),
                question: format!(
                    "mov al, {:#x}\nxor al, {:#x}\nadd al, 0x10\nЧему равен AL (hex)?",
                    a, k
                ),
                answer: format!("{:x}", c),
                explain: format!(
                    "{:#x} ^ {:#x} = {:#x}; {:#x} + 0x10 = {:#x}. Трасса пошагово: не в голове, а на бумаге!",
                    a, k, b, b, c
                ),
            }
        }
    }
}

/// Проверка ответа студента (hex без 0x, допуск 0x-префикса и регистра)
pub fn check_gen(task: &GenTask, student: &str) -> bool {
    let norm = |s: &str| s.trim().trim_start_matches("0x").to_lowercase();
    norm(student) == task.answer
}
