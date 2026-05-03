#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Eval harness standalone — invoca llama-helper.exe directamente para correr
los 12 scenarios contra el prompt actual de coach_simple_tick. Reporta
pass/fail + analisis para iterar prompt sin necesidad de la UI Tauri.

Uso:
    python -X utf8 scripts/eval_scenarios.py
"""
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).parent.parent
SIDECAR = ROOT / "frontend" / "src-tauri" / "binaries" / "llama-helper-x86_64-pc-windows-msvc.exe"
MODEL = Path(os.path.expandvars(r"%APPDATA%\com.maity.ai\models\summary\Qwen3-1.7B-Q4_K_M.gguf"))
SCENARIOS_DIR = ROOT / "frontend" / "src-tauri" / "scenarios"

SYSTEM_PROMPT = "Eres coach del vendedor. El vendedor esta hablando con el cliente. Te muestro el transcript y tu das UNA linea con lo que el vendedor debe decir AHORA."

USER_PROMPT_TEMPLATE = """Transcript (USUARIO = vendedor; INTERLOCUTOR = cliente):

{transcript}

Da UN tip CORTO que el vendedor diga AHORA al cliente.

Formato OBLIGATORIO (UNA sola linea, exactamente esta estructura):
Verbo: "frase entre comillas dobles"

donde:
- Verbo es UNA de estas palabras: Pregunta, Valida, Aclara, Refleja, Reconoce
- Despues del verbo va siempre el caracter dos puntos ":"
- Despues del ":" va siempre el caracter comilla doble " (de apertura)
- Adentro va la frase de 6 a 14 palabras que el vendedor dira
- Cierra con comilla doble "

Estos tres caracteres son OBLIGATORIOS: ":" + " (apertura) + " (cierre).
SI OLVIDAS LAS COMILLAS DOBLES, TU RESPUESTA NO SIRVE. Verifica antes de responder que tu output empiece con un verbo, despues ":" despues " y termine con ".

Como elegir el verbo:
- Cliente claro pero falta info -> Pregunta
- Cliente molesto, frustrado, triste, esceptico, con miedo o duda fuerte -> Valida
- Cliente vago o confuso -> Aclara
- Cliente con emocion fuerte que merece eco -> Refleja o Reconoce

Ejemplo del FORMATO exacto (no copies el contenido, inventalo segun el transcript):
Pregunta: "<frase de 6 a 14 palabras referida al transcript>"

Si la frase te sale con 15 o mas palabras, vuelve a escribirla con menos.
Si el transcript no permite un buen tip, responde solo: SIN_TIP

Tip:"""

VERBOS_VALIDOS = {
    "refleja", "espeja", "mirror", "etiqueta", "valida", "reconoce",
    "acompana", "acepta", "abraza", "asiente", "concede",
    "empatiza", "comprende", "humaniza", "personaliza", "conecta",
    "pregunta", "indaga", "explora", "aclara", "profundiza",
    "cuestiona", "verifica", "confirma", "escucha", "anota",
    "respira", "calma", "tranquiliza", "espera", "silencio",
    "reformula", "devuelve", "resume", "cita", "menciona",
}

VULGAR = ["caca", "pipi", "mierda", "puta", "pendej", "carajo",
          "inutil", "estupido", "idiota", "imbecil", "tonto"]


def load_scenarios():
    scenarios = []
    for path in sorted(SCENARIOS_DIR.glob("*.json")):
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
            scenarios.append((path.stem, data))
    return scenarios


def primera_palabra(tip):
    if not tip.strip():
        return ""
    word = tip.split()[0]
    while word and not word[-1].isalpha():
        word = word[:-1]
    while word and not word[0].isalpha():
        word = word[1:]
    return word.lower()


def evaluate(tip, expected_verbs):
    tip = tip.strip()
    word_count = len(tip.split())
    primera = primera_palabra(tip)
    verb_in_whitelist = primera in VERBOS_VALIDOS
    verb_match_expected = primera in [v.lower() for v in expected_verbs]
    has_colon = ":" in tip
    has_quotes = '"' in tip or "“" in tip or "”" in tip
    lower_full = tip.lower()
    is_sin_tip = "sin_tip" in lower_full or lower_full == "sin tip"
    is_vulgar = any(p in lower_full for p in VULGAR)
    too_long = word_count > 22
    too_short = word_count < 5

    rust_filter_pass = (
        verb_in_whitelist and has_colon and has_quotes
        and not is_sin_tip and not is_vulgar
        and not too_long and not too_short
    )
    eval_pass = rust_filter_pass and verb_match_expected

    notes = []
    if not verb_in_whitelist:
        notes.append("verbo no en whitelist (" + primera + ")")
    if not verb_match_expected:
        notes.append("verbo no esperado (esperaba " + str(expected_verbs) + ")")
    if not has_colon:
        notes.append("sin :")
    if not has_quotes:
        notes.append("sin comillas")
    if is_sin_tip:
        notes.append("SIN_TIP")
    if is_vulgar:
        notes.append("vulgar")
    if too_long:
        notes.append("muy largo (" + str(word_count) + "w)")
    if too_short:
        notes.append("muy corto (" + str(word_count) + "w)")

    return {
        "tip": tip,
        "word_count": word_count,
        "verb_in_whitelist": verb_in_whitelist,
        "verb_match_expected": verb_match_expected,
        "has_colon": has_colon,
        "has_quotes": has_quotes,
        "rust_filter_pass": rust_filter_pass,
        "eval_pass": eval_pass,
        "notes": " | ".join(notes) if notes else "OK",
    }


def call_sidecar(proc, prompt, max_tokens=80, timeout=60):
    msg = json.dumps({
        "type": "generate",
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": 0.4,
        "top_p": 0.9,
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


def main():
    if not SIDECAR.exists():
        print("ERROR: sidecar no existe: " + str(SIDECAR))
        sys.exit(1)
    if not MODEL.exists():
        print("ERROR: modelo no existe: " + str(MODEL))
        sys.exit(1)
    scenarios = load_scenarios()
    if not scenarios:
        print("ERROR: 0 scenarios")
        sys.exit(1)

    print("=== Eval Harness - " + str(len(scenarios)) + " scenarios ===")
    print("Sidecar: " + SIDECAR.name)
    print("Model: " + MODEL.name)
    print()

    proc = subprocess.Popen(
        [str(SIDECAR)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        encoding="utf-8",
        bufsize=1,
        env={**os.environ, "LLAMA_IDLE_TIMEOUT": "600"},
    )
    print("Sidecar PID " + str(proc.pid) + " arrancado")
    time.sleep(2)

    results = []
    total_latency = 0
    try:
        for fname, scenario in scenarios:
            prompt = USER_PROMPT_TEMPLATE.format(transcript=scenario["transcript"])
            full_prompt = (
                "<|im_start|>system\n" + SYSTEM_PROMPT + "<|im_end|>\n"
                "<|im_start|>user\n" + prompt + "\n\n/no_think<|im_end|>\n"
                "<|im_start|>assistant\n<think>\n\n</think>\n\n"
            )
            print("\n[" + fname + "] " + scenario["name"])
            t0 = time.time()
            tip = call_sidecar(proc, full_prompt, max_tokens=80, timeout=60)
            latency_ms = int((time.time() - t0) * 1000)
            total_latency += latency_ms

            for tag in ["</s>", "<|im_end|>", "<think>", "</think>", "Tip:"]:
                tip = tip.replace(tag, "")
            tip = tip.strip().lstrip("\n").strip()
            for line in tip.split("\n"):
                if line.strip():
                    tip = line.strip()
                    break

            ev = evaluate(tip, scenario["expected_verbs"])
            ev["scenario"] = scenario["name"]
            ev["category"] = scenario["category"]
            ev["expected_verbs"] = scenario["expected_verbs"]
            ev["latency_ms"] = latency_ms
            results.append(ev)
            mark = "PASS" if ev["eval_pass"] else "FAIL"
            print("  [" + mark + "] " + str(latency_ms) + "ms - " + ev["notes"])
            print("  TIP: " + tip)
    finally:
        try:
            proc.stdin.write(json.dumps({"type": "shutdown"}) + "\n")
            proc.stdin.flush()
            proc.wait(timeout=5)
        except Exception:
            proc.kill()

    passed = sum(1 for r in results if r["eval_pass"])
    rust_passed = sum(1 for r in results if r["rust_filter_pass"])
    n = len(results)
    avg = total_latency // n if n else 0

    print("\n" + "=" * 70)
    print("RESUMEN")
    print("=" * 70)
    print("Eval pass (formato + verbo esperado): " + str(passed) + "/" + str(n) + " (" + str(passed*100//n) + "%)")
    print("Rust filter pass (formato OK): " + str(rust_passed) + "/" + str(n) + " (" + str(rust_passed*100//n) + "%)")
    print("Avg latency: " + str(avg) + "ms")
    print("\nFAILURES:")
    for r in results:
        if not r["eval_pass"]:
            print("  - " + r["scenario"] + ": " + r["notes"])
            print("    TIP: " + r["tip"][:120])

    out = {
        "passed": passed,
        "rust_passed": rust_passed,
        "total": n,
        "avg_latency_ms": avg,
        "results": results,
    }
    out_path = ROOT / "scripts" / "last_eval_report.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(out, f, ensure_ascii=False, indent=2)
    print("\nReporte JSON: " + str(out_path))


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    main()
