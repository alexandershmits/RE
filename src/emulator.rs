//! Мини-эмулятор подмножества x86-64 для задач «Регистры и инструкции».
//! Ответы к задачам вычисляются, а не вводятся вручную — ключ не может разойтись с кодом.

use std::collections::HashMap;

const NAMES64: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15",
];
const NAMES32: [&str; 8] = ["eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi"];
const NAMES16: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];
const NAMES8: [&str; 4] = ["al", "cl", "dl", "bl"];
const MAX_STEPS: usize = 10_000;
const RSP: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Reg {
    idx: usize,
    bits: u32,
}

impl Reg {
    fn parse(name: &str) -> Option<Reg> {
        let n = name.trim().to_ascii_lowercase();
        let find = |table: &[&str], bits| {
            table
                .iter()
                .position(|r| *r == n)
                .map(|idx| Reg { idx, bits })
        };
        find(&NAMES64, 64)
            .or_else(|| find(&NAMES32, 32))
            .or_else(|| find(&NAMES16, 16))
            .or_else(|| find(&NAMES8, 8))
            .or_else(|| {
                // r8d / r8w / r8b
                let bits = match n.chars().last()? {
                    'd' => 32,
                    'w' => 16,
                    'b' => 8,
                    _ => return None,
                };
                let base = &n[..n.len() - 1];
                let numbered = base.len() >= 2 && base.as_bytes()[1].is_ascii_digit();
                NAMES64
                    .iter()
                    .position(|r| *r == base && numbered)
                    .map(|idx| Reg { idx, bits })
            })
    }

    fn mask(self) -> u64 {
        if self.bits == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        }
    }
}

#[derive(Clone, Debug)]
enum Operand {
    Reg(Reg),
    Imm(u64),
    Mem {
        base: Option<Reg>,
        index: Option<(Reg, u64)>,
        disp: i64,
    },
}

fn parse_imm(t: &str) -> Option<u64> {
    let t = t.trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t),
    };
    let v = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(&hex.replace('_', ""), 16).ok()?,
        None => t.parse::<u64>().ok()?,
    };
    Some(if neg { v.wrapping_neg() } else { v })
}

fn parse_operand(s: &str) -> Result<Operand, String> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        return parse_mem(inner);
    }
    if let Some(r) = Reg::parse(s) {
        return Ok(Operand::Reg(r));
    }
    parse_imm(s)
        .map(Operand::Imm)
        .ok_or_else(|| format!("неизвестный операнд: {s}"))
}

fn parse_mem(inner: &str) -> Result<Operand, String> {
    let compact: String = inner.chars().filter(|c| !c.is_whitespace()).collect();
    let mut terms: Vec<(i64, String)> = Vec::new();
    let (mut sign, mut cur) = (1i64, String::new());
    for ch in compact.chars() {
        if ch == '+' || ch == '-' {
            if !cur.is_empty() {
                terms.push((sign, std::mem::take(&mut cur)));
            }
            sign = if ch == '-' { -1 } else { 1 };
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        terms.push((sign, cur));
    }
    let (mut base, mut index, mut disp) = (None, None, 0i64);
    for (sg, term) in terms {
        if let Some((r, scale)) = term.split_once('*') {
            let reg = Reg::parse(r).ok_or_else(|| format!("регистр индекса: {r}"))?;
            let sc = parse_imm(scale).ok_or_else(|| format!("масштаб: {scale}"))?;
            index = Some((reg, sc));
        } else if let Some(reg) = Reg::parse(&term) {
            if base.is_none() {
                base = Some(reg);
            } else {
                index = Some((reg, 1));
            }
        } else if let Some(v) = parse_imm(&term) {
            disp = disp.wrapping_add(sg.wrapping_mul(v as i64));
        } else {
            return Err(format!("в адресе: {term}"));
        }
    }
    Ok(Operand::Mem { base, index, disp })
}

fn operand(args: &[String], op: &str, i: usize) -> Result<Operand, String> {
    let s = args
        .get(i)
        .ok_or_else(|| format!("{op}: не хватает операнда"))?;
    parse_operand(s)
}

fn reg_operand(args: &[String], op: &str, i: usize) -> Result<Reg, String> {
    match operand(args, op, i)? {
        Operand::Reg(r) => Ok(r),
        _ => Err(format!("{op}: операнд {} должен быть регистром", i + 1)),
    }
}

/// Состояние процессора: регистры общего назначения, ZF и стек.
#[derive(Default, Clone, Debug)]
pub struct Machine {
    regs: [u64; 16],
    zf: bool,
    stack: Vec<u64>,
}

impl Machine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, name: &str, value: u64) -> Result<(), String> {
        let r = Reg::parse(name).ok_or_else(|| format!("нет регистра {name}"))?;
        self.write(r, value);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<u64, String> {
        let r = Reg::parse(name).ok_or_else(|| format!("нет регистра {name}"))?;
        Ok(self.regs[r.idx] & r.mask())
    }

    /// Запись с семантикой x86-64: 32-битная запись обнуляет старшую половину, 8/16-битная сохраняет её.
    fn write(&mut self, r: Reg, v: u64) {
        let old = self.regs[r.idx];
        self.regs[r.idx] = match r.bits {
            64 => v,
            32 => v & 0xFFFF_FFFF,
            _ => (old & !r.mask()) | (v & r.mask()),
        };
    }

    fn read(&self, op: &Operand) -> Result<u64, String> {
        match op {
            Operand::Reg(r) => Ok(self.regs[r.idx] & r.mask()),
            Operand::Imm(v) => Ok(*v),
            Operand::Mem { .. } => Err("чтение памяти не поддерживается".into()),
        }
    }

    fn address(&self, op: &Operand) -> Result<u64, String> {
        let Operand::Mem { base, index, disp } = op else {
            return Err("ожидался адрес вида [..]".into());
        };
        let mut a = *disp as u64;
        if let Some(b) = base {
            a = a.wrapping_add(self.regs[b.idx]);
        }
        if let Some((i, scale)) = index {
            a = a.wrapping_add(self.regs[i.idx].wrapping_mul(*scale));
        }
        Ok(a)
    }

    /// Выполняет программу; метки `name:` и переходы `jz/jnz/jmp` поддерживаются.
    pub fn run(&mut self, code: &[String]) -> Result<(), String> {
        let mut prog: Vec<(String, Vec<String>)> = Vec::new();
        let mut labels: HashMap<String, usize> = HashMap::new();
        for raw in code {
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_suffix(':') {
                labels.insert(name.trim().to_string(), prog.len());
                continue;
            }
            let (op, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            let args = rest
                .split(',')
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty())
                .collect();
            prog.push((op.to_ascii_lowercase(), args));
        }
        let (mut pc, mut steps) = (0usize, 0usize);
        while pc < prog.len() {
            steps += 1;
            if steps > MAX_STEPS {
                return Err("слишком много шагов (бесконечный цикл?)".into());
            }
            let (op, args) = &prog[pc];
            pc += 1;
            if let Some(target) = self.step(op, args, &labels)? {
                pc = target;
            }
        }
        Ok(())
    }

    fn step(
        &mut self,
        op: &str,
        args: &[String],
        labels: &HashMap<String, usize>,
    ) -> Result<Option<usize>, String> {
        match op {
            "nop" => {}
            "mov" => {
                let d = reg_operand(args, op, 0)?;
                let v = self.read(&operand(args, op, 1)?)?;
                self.write(d, v);
            }
            "lea" => {
                let d = reg_operand(args, op, 0)?;
                let a = self.address(&operand(args, op, 1)?)?;
                self.write(d, a);
            }
            "add" | "sub" | "xor" | "and" | "or" => {
                let d = reg_operand(args, op, 0)?;
                let a = self.regs[d.idx] & d.mask();
                let b = self.read(&operand(args, op, 1)?)? & d.mask();
                let v = match op {
                    "add" => a.wrapping_add(b),
                    "sub" => a.wrapping_sub(b),
                    "xor" => a ^ b,
                    "and" => a & b,
                    _ => a | b,
                } & d.mask();
                self.write(d, v);
                self.zf = v == 0;
            }
            "shl" | "shr" => {
                let d = reg_operand(args, op, 0)?;
                let count =
                    self.read(&operand(args, op, 1)?)? & if d.bits == 64 { 0x3F } else { 0x1F };
                let a = self.regs[d.idx] & d.mask();
                let shifted = if op == "shl" {
                    a.checked_shl(count as u32)
                } else {
                    a.checked_shr(count as u32)
                };
                let v = shifted.unwrap_or(0) & d.mask();
                if count != 0 {
                    self.zf = v == 0;
                }
                self.write(d, v);
            }
            "inc" | "dec" => {
                let d = reg_operand(args, op, 0)?;
                let a = self.regs[d.idx] & d.mask();
                let v = if op == "inc" {
                    a.wrapping_add(1)
                } else {
                    a.wrapping_sub(1)
                } & d.mask();
                self.write(d, v);
                self.zf = v == 0;
            }
            "push" => {
                let v = self.read(&operand(args, op, 0)?)?;
                self.stack.push(v);
                self.regs[RSP] = self.regs[RSP].wrapping_sub(8);
            }
            "pop" => {
                let d = reg_operand(args, op, 0)?;
                let v = self.stack.pop().ok_or("pop из пустого стека")?;
                self.regs[RSP] = self.regs[RSP].wrapping_add(8);
                self.write(d, v);
            }
            "cmp" | "test" => {
                let d = reg_operand(args, op, 0)?;
                let a = self.regs[d.idx] & d.mask();
                let b = self.read(&operand(args, op, 1)?)? & d.mask();
                self.zf = if op == "cmp" { a == b } else { a & b == 0 };
            }
            "jz" | "je" | "jnz" | "jne" | "jmp" => {
                let label = args.first().ok_or_else(|| format!("{op}: нужна метка"))?;
                let target = *labels
                    .get(label.as_str())
                    .ok_or_else(|| format!("нет метки {label}"))?;
                let take = match op {
                    "jz" | "je" => self.zf,
                    "jnz" | "jne" => !self.zf,
                    _ => true,
                };
                if take {
                    return Ok(Some(target));
                }
            }
            other => return Err(format!("инструкция не поддерживается: {other}")),
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(code: &[&str], init: &[(&str, u64)]) -> Machine {
        let mut m = Machine::new();
        for (r, v) in init {
            m.set(r, *v).unwrap();
        }
        let code: Vec<String> = code.iter().map(|s| s.to_string()).collect();
        m.run(&code).unwrap();
        m
    }

    #[test]
    fn lea_computes_address_without_touching_memory() {
        let m = run(
            &[
                "mov rax, 0x10",
                "mov rbx, 0x4",
                "lea rcx, [rax + rbx*2]",
                "add rax, rbx",
            ],
            &[],
        );
        assert_eq!(
            (m.get("rax"), m.get("rbx"), m.get("rcx")),
            (Ok(0x14), Ok(4), Ok(0x18))
        );
    }

    #[test]
    fn writing_eax_zeroes_the_upper_half() {
        let m = run(&["mov rax, 0xFFFFFFFFFFFFFFFF", "mov eax, 0x1"], &[]);
        assert_eq!(m.get("rax"), Ok(1));
    }

    #[test]
    fn eight_bit_writes_keep_upper_bits() {
        let m = run(&["mov rax, 0x1234", "mov al, 0xFF"], &[]);
        assert_eq!(m.get("rax"), Ok(0x12FF));
        let m = run(&["mov rax, 0x1234", "mov al, 0xFF", "add al, 1"], &[]);
        assert_eq!(m.get("rax"), Ok(0x1200));
    }

    #[test]
    fn stack_is_lifo() {
        let m = run(
            &[
                "mov rax, 0xAAAA",
                "mov rbx, 0xBBBB",
                "push rax",
                "push rbx",
                "pop rcx",
                "pop rdx",
            ],
            &[],
        );
        assert_eq!((m.get("rcx"), m.get("rdx")), (Ok(0xBBBB), Ok(0xAAAA)));
    }

    #[test]
    fn push_and_pop_move_rsp_by_eight() {
        let m = run(
            &["mov rsp, 0x1000", "mov rax, 7", "push rax", "push rax"],
            &[],
        );
        assert_eq!(m.get("rsp"), Ok(0x1000 - 16));
        let m = run(
            &["mov rsp, 0x1000", "mov rax, 7", "push rax", "pop rbx"],
            &[],
        );
        assert_eq!((m.get("rsp"), m.get("rbx")), (Ok(0x1000), Ok(7)));
    }

    #[test]
    fn cmp_and_jz_skip_the_next_instruction() {
        let m = run(
            &[
                "mov eax, 0x5",
                "cmp eax, 0x5",
                "jz done",
                "mov eax, 0xFF",
                "done:",
            ],
            &[],
        );
        assert_eq!(m.get("eax"), Ok(5));
        let m = run(
            &[
                "mov eax, 0x5",
                "cmp eax, 0x6",
                "jz done",
                "mov eax, 0xFF",
                "done:",
            ],
            &[],
        );
        assert_eq!(m.get("eax"), Ok(0xFF));
    }

    #[test]
    fn shift_by_cl_then_inc() {
        let m = run(
            &[
                "mov rax, 0xFF",
                "xor rbx, rbx",
                "mov rcx, 0x3",
                "shl rax, cl",
                "inc rax",
            ],
            &[("rbx", 0x77)],
        );
        assert_eq!(
            (m.get("rax"), m.get("rbx"), m.get("rcx")),
            (Ok(0x7F9), Ok(0), Ok(3))
        );
    }

    #[test]
    fn errors_are_reported_not_swallowed() {
        let mut m = Machine::new();
        let bad = |code: &[&str]| code.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(m
            .run(&bad(&["frobnicate rax"]))
            .unwrap_err()
            .contains("не поддерживается"));
        assert!(m
            .run(&bad(&["jmp nowhere"]))
            .unwrap_err()
            .contains("нет метки"));
        assert!(m
            .run(&bad(&["loop:", "jmp loop"]))
            .unwrap_err()
            .contains("слишком много"));
        assert!(m
            .run(&bad(&["pop rax"]))
            .unwrap_err()
            .contains("пустого стека"));
        assert!(m.set("foo", 1).is_err());
    }
}
