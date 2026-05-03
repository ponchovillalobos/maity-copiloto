#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Prompt Lab Runner — corre el prompt actual sobre las fixtures de
D:\\Maity_Desktop\\scripts\\prompt_lab\\fixtures simulando ventanas de 15s
de conversacion. Persiste cada tip en SQLite tabla prompt_lab_runs para
visualizar en el dashboard web (http://localhost:3119/lab).

Modelo de simulacion: 130 palabras/minuto (espanol conversacional).
Asi 15s = ~32 palabras de conversacion.

Uso: python -X utf8 scripts/prompt_lab_runner.py [--fixtures N] [--max-windows M]
"""
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).parent.parent
SIDECAR = ROOT / "frontend" / "src-tauri" / "binaries" / "llama-helper-x86_64-pc-windows-msvc.exe"
MODEL = Path(os.path.expandvars(r"%APPDATA%\com.maity.ai\models\summary\Qwen3-1.7B-Q4_K_M.gguf"))
DB_PATH = Path(os.path.expandvars(r"%APPDATA%\com.maity.ai\meeting_minutes.sqlite"))
FIXTURES_DIR = Path(r"D:\Maity_Desktop\scripts\prompt_lab\fixtures")

PROMPT_VERSION = "v32.0"

# Prompt v32.0 — debe ser equivalente al de commands.rs:247-... (categorías)
SYSTEM_PROMPT = """/no_think
Eres Maity. Le hablas al oído al USUARIO mientras está en una conversación con el INTERLOCUTOR. Eres su coach humano susurrándole qué hacer ahora.

Lee el transcript y dale UN tip cortito, humano, directo. Como un amigo experto al lado.

Mira señales OBVIAS, no infieras de más:
- ¿Quién habla más? Si el USUARIO domina, hay que cederle turno al otro.
- ¿El INTERLOCUTOR usó palabras de molestia, duda, miedo o interés? Reconócelo.
- ¿Falta info importante para avanzar? Toca preguntar.

CATEGORÍAS (elige UNA):

RESPIRA — el USUARIO se aceleró, sonó defensivo o tenso.
PAUSA — el USUARIO lleva varios turnos seguidos sin que hable el INTERLOCUTOR.
PREGUNTA — falta información o el INTERLOCUTOR no ha participado lo suficiente.
ESCUCHA — el INTERLOCUTOR está hablando largo, desahogándose o explicando algo.
VALIDA — el INTERLOCUTOR mostró molestia, duda, miedo o desacuerdo.
AVANZA — el INTERLOCUTOR mostró interés claro o pidió siguientes pasos.

FORMATO:
CATEGORIA: tip

REGLAS:
- Máximo 8 palabras en el tip.
- Trato directo (tú), humano, cero corporativo.
- Puedes sugerir frases cortas tipo "estoy de acuerdo", "tienes razón", "entiendo tu punto" cuando aplique.
- Sin nombres, cifras ni promesas inventadas.
- Si el INTERLOCUTOR está molesto: prioriza VALIDA o ESCUCHA antes que cualquier otra.
- Si no hay señal clara: SIN_TIP

EJEMPLOS:

Transcript:
USUARIO: Tenemos esto, esto, esto y también esto otro
USUARIO: Y además incluye soporte y dashboard
→ PAUSA: Cédele el turno con una pregunta.

Transcript:
USUARIO: Pero no, no es así, déjame explicarte
→ RESPIRA: Bájale. No es contra ti.

Transcript:
INTERLOCUTOR: Esto no funciona como me prometieron
→ VALIDA: Dile "tienes razón, déjame entender".

Transcript:
INTERLOCUTOR: Estoy harto, llevamos meses así
→ VALIDA: Reconoce su molestia primero.

Transcript:
INTERLOCUTOR: La verdad llevamos años batallando con esto y nadie nos ha podido ayudar bien
→ ESCUCHA: No interrumpas. Déjalo terminar.

Transcript:
INTERLOCUTOR: No sé, depende de varias cosas
→ PREGUNTA: Pídele un ejemplo concreto.

Transcript:
INTERLOCUTOR: Me interesa, ¿cómo seguimos?
→ AVANZA: Agenda el siguiente paso ya."""

USER_PROMPT_TEMPLATE = """Transcript (USUARIO = nuestro comunicador; INTERLOCUTOR = la otra parte):
{transcript}

Tip:"""

WINDOW_SECONDS = 15
WORDS_PER_MINUTE = 130
WORDS_PER_WINDOW = (WORDS_PER_MINUTE * WINDOW_SECONDS) // 60  # ~32 palabras
WINDOW_CHAR_CAP = 800

CATEGORIAS_VALIDAS = {"RESPIRA", "PAUSA", "PREGUNTA", "ESCUCHA", "VALIDA", "AVANZA"}


def parse_fixture(path):
    """Lee fixture .txt formato '[role] texto'. Devuelve lista de (role_label, text, word_count)."""
    turns = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            m = re.match(r"\[(\w+)\]\s*(.+)", line.strip())
            if m:
                role = m.group(1)
                text = m.group(2)
                role_label = "USUARIO" if role == "user" else "INTERLOCUTOR" if role == "interlocutor" else role.upper()
                wc = len(text.split())
                turns.append((role_label, text, wc))
    return turns


def build_windows(turns):
    """Genera ventanas progresivas de ~32 palabras (15s) cumulativas. Cada
    ventana incluye desde el inicio hasta el turno cuyo acumulado pasa el threshold."""
    windows = []
    cumulative_words = 0
    last_window_words = 0
    current_lines = []
    window_idx = 0
    for role, text, wc in turns:
        current_lines.append(f"{role}: {text}")
        cumulative_words += wc
        if cumulative_words - last_window_words >= WORDS_PER_WINDOW:
            window_idx += 1
            window_text = "\n".join(current_lines)
            if len(window_text) > WINDOW_CHAR_CAP:
                window_text = window_text[-WINDOW_CHAR_CAP:]
            windows.append({
                "idx": window_idx,
                "seconds": window_idx * WINDOW_SECONDS,
                "text": window_text,
                "chars": len(window_text),
            })
            last_window_words = cumulative_words
    return windows


def primera_palabra(tip):
    if not tip.strip():
        return ""
    word = tip.split()[0]
    while word and not word[-1].isalpha():
        word = word[:-1]
    while word and not word[0].isalpha():
        word = word[1:]
    return word.lower()


def evaluate(tip):
    """v32.0: valida formato CATEGORIA: tip (1..=8 palabras en tip, sin comillas)."""
    tip_clean = tip.strip().lstrip("→").lstrip("->").strip()
    is_sin_tip = tip_clean.upper() in ("SIN_TIP", "SIN TIP")
    if is_sin_tip:
        return {"verb": "SIN_TIP", "has_quotes": False, "word_count": 0, "passed": False, "notes": "SIN_TIP"}
    if ":" not in tip_clean:
        return {"verb": "?", "has_quotes": False, "word_count": len(tip_clean.split()), "passed": False, "notes": "sin :"}
    cat_part, rest = tip_clean.split(":", 1)
    cat_upper = cat_part.strip().upper()
    text = rest.strip().strip('"').strip()
    word_count = len(text.split())
    cat_valida = cat_upper in CATEGORIAS_VALIDAS
    too_short = word_count < 1
    too_long = word_count > 8
    passed = cat_valida and not too_short and not too_long
    notes = []
    if not cat_valida:
        notes.append(f"categoria invalida ({cat_upper})")
    if too_short:
        notes.append("texto vacio")
    if too_long:
        notes.append(f"largo ({word_count}w)")
    return {
        "verb": cat_upper.lower(),  # reusamos el campo `verb` en DB para guardar la categoría
        "has_quotes": False,
        "word_count": word_count,
        "passed": passed,
        "notes": " | ".join(notes) if notes else "OK",
    }


def call_sidecar(proc, prompt, max_tokens=35, timeout=60):
    msg = json.dumps({
        "type": "generate",
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": 0.3,
        "top_p": 0.85,
        "model_path": str(MODEL),
    })
    proc.stdin.write(msg + "\n")
    proc.stdin.flush()
    start = time.time()
    while time.time() - start < timeout:
        line = proc.stdout.readline()
        if not line:
            time.sleep(0.05)
            continue
        line = line.strip()
        if not line:
            continue
        try:
            resp = json.loads(line)
        except json.JSONDecodeError:
            continue
        t = resp.get("type")
        if t == "response":
            return resp.get("text", "")
        if t == "error":
            return "[ERROR] " + str(resp.get("message"))
    return "[TIMEOUT]"


def clean_tip(raw):
    """Strip qwen3 thinking + markdown + prefijos."""
    # Strip <think>...</think>
    while "<think>" in raw and "</think>" in raw:
        s = raw.find("<think>")
        e = raw.find("</think>")
        if s < e:
            raw = raw[:s] + raw[e + len("</think>"):]
        else:
            break
    if "<think>" in raw:
        raw = raw[:raw.find("<think>")]
    raw = raw.strip()
    # Strip markdown fences
    if raw.startswith("```"):
        raw = raw.split("\n", 1)[-1] if "\n" in raw else raw
    raw = raw.replace("```json", "").replace("```", "").strip()
    # Tag end
    raw = raw.replace("<|im_end|>", "").replace("</s>", "").strip()
    # Primera linea no vacia
    for line in raw.split("\n"):
        if line.strip():
            raw = line.strip()
            break
    # Strip prefijos
    for pre in ["Tip:", "TIP:", "tip:", "Consejo:", "Sugerencia:"]:
        if raw.startswith(pre):
            raw = raw[len(pre):].strip()
            break
    return raw


def insert_run(db, row):
    db.execute(
        """INSERT INTO prompt_lab_runs
           (run_id, fixture_name, window_idx, window_seconds, transcript_chars,
            transcript_window, tip_raw, tip_clean, verb, has_quotes, word_count,
            latency_ms, prompt_version, model, passed, notes)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        (
            row["run_id"], row["fixture_name"], row["window_idx"], row["window_seconds"],
            row["transcript_chars"], row["transcript_window"], row["tip_raw"], row["tip_clean"],
            row["verb"], 1 if row["has_quotes"] else 0, row["word_count"],
            row["latency_ms"], row["prompt_version"], row["model"],
            1 if row["passed"] else 0, row["notes"],
        ),
    )
    db.commit()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fixtures", type=int, default=10, help="Num fixtures a procesar (def 10)")
    ap.add_argument("--max-windows", type=int, default=8, help="Max ventanas por fixture (def 8)")
    args = ap.parse_args()

    if not SIDECAR.exists() or not MODEL.exists() or not FIXTURES_DIR.exists():
        print("ERROR: paths invalidos.\n  sidecar:", SIDECAR, SIDECAR.exists(), "\n  model:", MODEL, MODEL.exists(), "\n  fixtures:", FIXTURES_DIR, FIXTURES_DIR.exists())
        sys.exit(1)

    fixture_paths = sorted(FIXTURES_DIR.glob("*.txt"))[:args.fixtures]
    if not fixture_paths:
        print("ERROR: no hay fixtures .txt")
        sys.exit(1)

    run_id = "run-" + uuid.uuid4().hex[:8]
    print(f"=== Prompt Lab {PROMPT_VERSION} — run {run_id} ===")
    print(f"Fixtures: {len(fixture_paths)}, max windows/fixture: {args.max_windows}")
    print(f"Window sim: {WINDOW_SECONDS}s = ~{WORDS_PER_WINDOW} palabras")
    print()

    proc = subprocess.Popen(
        [str(SIDECAR)],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        text=True, encoding="utf-8", bufsize=1,
        env={**os.environ, "LLAMA_IDLE_TIMEOUT": "1800"},
    )
    print(f"Sidecar PID {proc.pid}")
    time.sleep(2)

    db = sqlite3.connect(str(DB_PATH))

    total_tips = 0
    total_passed = 0
    total_latency = 0

    try:
        for fpath in fixture_paths:
            fname = fpath.stem
            turns = parse_fixture(fpath)
            if not turns:
                continue
            windows = build_windows(turns)[:args.max_windows]
            print(f"\n[{fname}] {len(windows)} ventanas")

            for w in windows:
                full_prompt = (
                    "<|im_start|>system\n" + SYSTEM_PROMPT + "<|im_end|>\n"
                    "<|im_start|>user\n" + USER_PROMPT_TEMPLATE.format(transcript=w["text"]) + "\n\n/no_think<|im_end|>\n"
                    "<|im_start|>assistant\n<think>\n\n</think>\n\n"
                )
                t0 = time.time()
                tip_raw = call_sidecar(proc, full_prompt, max_tokens=35, timeout=60)
                latency_ms = int((time.time() - t0) * 1000)
                total_latency += latency_ms
                tip_clean = clean_tip(tip_raw)
                ev = evaluate(tip_clean)
                total_tips += 1
                if ev["passed"]:
                    total_passed += 1

                insert_run(db, {
                    "run_id": run_id,
                    "fixture_name": fname,
                    "window_idx": w["idx"],
                    "window_seconds": w["seconds"],
                    "transcript_chars": w["chars"],
                    "transcript_window": w["text"],
                    "tip_raw": tip_raw[:500],
                    "tip_clean": tip_clean,
                    "verb": ev["verb"],
                    "has_quotes": ev["has_quotes"],
                    "word_count": ev["word_count"],
                    "latency_ms": latency_ms,
                    "prompt_version": PROMPT_VERSION,
                    "model": "qwen3:1.7b",
                    "passed": ev["passed"],
                    "notes": ev["notes"],
                })

                mark = "PASS" if ev["passed"] else "FAIL"
                print(f"  w{w['idx']} ({w['seconds']}s, {w['chars']}ch) [{mark}] {latency_ms}ms - {tip_clean[:80]}")

    finally:
        try:
            proc.stdin.write(json.dumps({"type": "shutdown"}) + "\n")
            proc.stdin.flush()
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
        db.close()

    print()
    print("=" * 60)
    print(f"Total tips: {total_tips}, PASS {total_passed}/{total_tips} ({total_passed*100//max(1,total_tips)}%)")
    print(f"Avg latency: {total_latency // max(1,total_tips)}ms")
    print(f"Run id: {run_id}")
    print(f"Dashboard: http://localhost:3119/lab")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    main()
