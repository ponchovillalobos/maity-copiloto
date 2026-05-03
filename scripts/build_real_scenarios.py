#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Convierte transcripts reales (D:\\Poncho\\Videos\\Edicion-Claude\\output) en
scenarios JSON para el harness. Toma los ULTIMOS 8 turnos de cada conversacion
(momento donde el vendedor/agente necesitaria mas un tip de coach).

Genera scenarios en frontend/src-tauri/scenarios/real_*.json.
Acepta CUALQUIER verbo de la whitelist (expected_verbs = todos), por lo que
el eval solo verifica formato + contextual.

Uso: python -X utf8 scripts/build_real_scenarios.py
"""
import json
import re
import sys
from pathlib import Path

SOURCE = Path(r"D:\Poncho\Videos\Edicion-Claude\output")
OUT_DIR = Path(__file__).parent.parent / "frontend" / "src-tauri" / "scenarios"

# Verbos amplios — cualquier verbo del coach es valido para scenarios reales,
# porque puede haber multiples respuestas validas a una situacion.
ALL_VERBS = ["pregunta", "valida", "aclara", "refleja", "reconoce"]

WINDOW_LAST_N_TURNS = 8


def parse_txt(path):
    """Lee transcript .txt formato '[role] texto'. Devuelve lista de tuplas."""
    turns = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            m = re.match(r"\[(\w+)\]\s*(.+)", line.strip())
            if m:
                role = m.group(1)
                text = m.group(2)
                # Mapea: user -> USUARIO (vendedor), interlocutor -> INTERLOCUTOR (cliente)
                if role == "user":
                    role_label = "USUARIO"
                elif role == "interlocutor":
                    role_label = "INTERLOCUTOR"
                else:
                    role_label = role.upper()
                turns.append((role_label, text))
    return turns


def build_window(turns, last_n):
    """Toma los ultimos N turnos. Si el ultimo es del USUARIO, retrocede 1
    para que el window termine con INTERLOCUTOR (el coach va a sugerir como
    responder al cliente)."""
    if not turns:
        return []
    end = len(turns)
    # Si el ultimo turno es del USUARIO, recorta 1 para que termine en cliente
    if turns[-1][0] == "USUARIO":
        end -= 1
    start = max(0, end - last_n)
    return turns[start:end]


def turns_to_text(turns):
    return "\n".join(f"{role}: {text}" for role, text in turns)


def main():
    if not SOURCE.exists():
        print("ERROR: source dir no existe: " + str(SOURCE))
        sys.exit(1)

    folders = sorted([p for p in SOURCE.iterdir() if p.is_dir()])
    if not folders:
        print("ERROR: 0 folders")
        sys.exit(1)

    created = 0
    for folder in folders:
        # Buscar el .txt principal (mismo nombre que carpeta)
        txt = folder / (folder.name + ".txt")
        if not txt.exists():
            continue
        try:
            turns = parse_txt(txt)
        except Exception as e:
            print(f"  skip {folder.name}: {e}")
            continue
        if len(turns) < 4:
            print(f"  skip {folder.name}: muy pocos turnos ({len(turns)})")
            continue
        window = build_window(turns, WINDOW_LAST_N_TURNS)
        if not window or window[-1][0] != "INTERLOCUTOR":
            # No hay window util terminando en cliente
            continue

        scenario = {
            "name": folder.name,
            "category": "real_" + folder.name.split("_")[0],
            "expected_intent": "tip empatico que ayude al vendedor a responder al cliente",
            "expected_verbs": ALL_VERBS,  # cualquier verbo valido
            "transcript": turns_to_text(window),
        }

        out_path = OUT_DIR / f"real_{folder.name}.json"
        with open(out_path, "w", encoding="utf-8") as f:
            json.dump(scenario, f, ensure_ascii=False, indent=2)
        print(f"  + {out_path.name} ({len(window)} turns)")
        created += 1

    print(f"\nCreados {created} scenarios reales en {OUT_DIR}")


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
