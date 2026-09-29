#!/usr/bin/env python3
"""Adversarial loop: генерация параметризованных челленджей (string.Template).
Использование:
  python3 challenge_generator.py gen1 out_dir [elf|exe]
  python3 challenge_generator.py all out_dir
"""
import os, random, sys, subprocess, json
from string import Template

TEMPLATES = {
 "gen1": {
  "level": 2, "title": "XOR-число [random]",
  "desc": "x ^ CONST == TARGET. CONST и TARGET меняются при каждой генерации!",
  "tmpl": Template("""#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  if ((x ^ $K1) == $T) { puts("Correct! FLAG{$flag}"); return 0; }
  puts("Wrong"); return 1; }"""),
 },
 "gen2": {
  "level": 2, "title": "Сдвиг [random]",
  "desc": "(x >> S) == T. S и T случайны.",
  "tmpl": Template("""#include <stdio.h>
int main(void){ unsigned x;
  printf("number: "); scanf("%u", &x);
  if ((x >> $S) == $T) { puts("Correct! FLAG{$flag}"); return 0; }
  puts("Wrong"); return 1; }"""),
 },
 "gen3": {
  "level": 3, "title": "Хеш-31 [random]",
  "desc": "h = h*31 + c, seed случайный. Пароль — случайное слово+цифры.",
  "tmpl": Template("""#include <stdio.h>
#include <string.h>
int main(void){ char buf[32]; unsigned sum = $SEED;
  printf("password: "); scanf("%31s", buf);
  for (int i = 0; buf[i]; i++) sum = sum * 31 + (unsigned char)buf[i];
  if (sum == ${TARGET}u) { puts("Correct! FLAG{$flag}"); return 0; }
  puts("Wrong"); return 1; }"""),
 },
 "gen4": {
  "level": 3, "title": "Позиционный XOR [random]",
  "desc": "target[i] = pw[i] ^ (BASE + i). BASE случайный, пароль — случайное слово.",
  "tmpl": Template("""#include <stdio.h>
#include <string.h>
int main(void){ char buf[32]; unsigned char out[32]; int n;
  printf("password: "); scanf("%31s", buf); n = strlen(buf);
  if (n != $N) { puts("Wrong"); return 1; }
  for (int i = 0; i < n; i++) out[i] = buf[i] ^ ($BASE + i);
  const unsigned char target[$N] = {$TARGETS};
  if (!memcmp(out, target, $N)) { puts("Correct! FLAG{$flag}"); return 0; }
  puts("Wrong"); return 1; }"""),
 },
 "gen5": {
  "level": 4, "title": "Двухступенчатый [random]",
  "desc": "stage1: строка; stage2: хеш h*MULT+c. Всё рандомно.",
  "tmpl": Template("""#include <stdio.h>
#include <string.h>
int main(void){ char s1[16], s2[16];
  printf("stage1: "); scanf("%15s", s1);
  if (strcmp(s1, "$S1")) { puts("Wrong stage1"); return 1; }
  printf("stage2: "); scanf("%15s", s2);
  unsigned h = 0;
  for (int i = 0; s2[i]; i++) h = h * $MULT + s2[i];
  if (h == ${H2}) { puts("Correct! FLAG{$flag}"); return 0; }
  puts("Wrong stage2"); return 1; }"""),
 },
}

WORDS = ["reverse","engineer","binary","debugger","ghidra","ida","assemble","pointer",
         "stackframe","payload","malware","sandbox","entropy","xref","decompile",
         "export","import","section","entrypoint","opcode","register","compiler"]

def gen_xor_num(rng):
    k1 = rng.randint(0x100, 0xFFFF)
    t = rng.randint(0x100, 0xFFFF)
    x = k1 ^ t
    flag = f"g{x % 99991:x}{k1 % 9973:x}"
    return {"K1": f"{k1:#x}", "T": f"{t:#x}", "flag": flag}, f"ответ: число {x}", flag

def gen_shift(rng):
    s = rng.randint(1, 15)
    t = rng.randint(0x100, 0xFFFF)
    x = t << s
    flag = f"s{s:02d}x{t % 99991:x}"
    return {"S": str(s), "T": f"{t:#x}", "flag": flag}, f"ответ: число {x}", flag

def gen_hash31(rng):
    seed = rng.randint(1, 0xFFFF)
    word = rng.choice(WORDS) + str(rng.randint(10, 99))
    h = seed
    for c in word.encode():
        h = (h * 31 + c) & 0xFFFFFFFF
    flag = f"h{h % 999983:x}"
    return {"SEED": str(seed), "TARGET": f"{h:#x}", "flag": flag}, f"пароль: {word}", flag

def gen_xorpos(rng):
    n = rng.randint(6, 10)
    base = rng.randint(0x30, 0x60)
    pw = rng.choice(WORDS)[:n]
    while len(pw) < n:
        pw += rng.choice("0123456789abcdef")
    targets = ",".join(str(ord(c) ^ (base + i)) for i, c in enumerate(pw))
    flag = f"x{base:02x}n{n}"
    return {"N": str(n), "BASE": str(base), "TARGETS": targets, "flag": flag}, f"пароль: {pw}", flag

def gen_2stage(rng):
    s1 = rng.choice(WORDS)[:6]
    mult = rng.choice([0x1F, 0x21, 0x25, 0x33])
    s2 = rng.choice(WORDS)[:6] + str(rng.randint(1, 9))
    h = 0
    for c in s2.encode():
        h = (h * mult + c) & 0xFFFFFFFF
    flag = f"2s{h % 99991:x}"
    return {"S1": s1, "MULT": str(mult), "H2": f"{h:#x}", "flag": flag}, f"stage1: {s1}; stage2: {s2}", flag

BUILDERS = {"gen1": gen_xor_num, "gen2": gen_shift, "gen3": gen_hash31,
            "gen4": gen_xorpos, "gen5": gen_2stage}

def build(cid, out_dir, seed=None, platform="elf"):
    rng = random.Random(seed) if seed is not None else random.SystemRandom()
    t = TEMPLATES[cid]
    params, solve_hint, flag = BUILDERS[cid](rng)
    src = t["tmpl"].substitute(**params)
    out = os.path.abspath(out_dir)
    os.makedirs(out, exist_ok=True)
    src_path = os.path.join(out, f"{cid}.c")
    open(src_path, "w").write(src)
    cc = "gcc" if platform == "elf" else "x86_64-w64-mingw32-gcc"
    ext = "" if platform == "elf" else ".exe"
    bin_path = os.path.join(out, cid + ext)
    r = subprocess.run([cc, "-O0", "-m64", src_path, "-o", bin_path, "-w"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr)
    meta = {"id": cid, "title": t["title"], "desc": t["desc"], "flag": flag,
            "solve_hint": solve_hint,
            "params": {k: str(v) for k, v in params.items()}}
    json.dump(meta, open(os.path.join(out, f"{cid}.meta.json"), "w"),
              ensure_ascii=False, indent=1)
    return meta

if __name__ == "__main__":
    cid, out = sys.argv[1], sys.argv[2]
    platform = sys.argv[3] if len(sys.argv) > 3 else "elf"
    ids = list(TEMPLATES) if cid == "all" else [cid]
    for c in ids:
        m = build(c, out, platform=platform)
        print("OK", c, "flag:", m["flag"])
