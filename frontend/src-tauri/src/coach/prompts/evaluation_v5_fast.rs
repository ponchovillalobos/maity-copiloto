//! Prompt eval v5-fast — versión condensada (4KB vs 14KB v4) optimizada para
//! velocidad. Genera JSON estructurado mínimo viable: secciones críticas
//! identificacion, meta, resumen, dimensiones, recomendaciones.
//!
//! Target: eval_ms < 180s en CPU (vs ~430s con v4 completo).
//! Trade-off: omite timeline, por_hablante detallado, empatía granular.
//! Usar v5_fast para iteración rápida; v4 completo para release final.

pub const PROMPT_VERSION_FAST: &str = "v5-fast";

pub const EVALUATION_V5_FAST_PROMPT: &str = r#"Eres coach de comunicación en español. Analiza la transcripción y genera SOLO JSON válido.

REGLAS:
- Solo comillas simples (') dentro de strings. Nunca dobles internas.
- Sin markdown, sin texto fuera del JSON. Sin comas trailing.
- Termina siempre el JSON con } cerrado.

ESTRUCTURA REQUERIDA:
{
  "identificacion": {
    "nombre_sesion": "<descripción 5 palabras>",
    "fecha_analisis": "2026-04-30",
    "version_prompt": "v5-fast",
    "idioma": "es"
  },
  "contexto": {
    "tipo_comunicacion": "<venta|atención|reunión|coaching|entrevista|presentación>",
    "relacion": "<asesor-cliente|líder-equipo|mentor-mentee>",
    "objetivo_declarado": "<qué busca el user>",
    "objetivo_real_inferido": "<qué realmente buscaba>",
    "alineacion_objetivo": <0.0-1.0>
  },
  "meta": {
    "tipo": "<categoría>",
    "duracion_minutos": <numero>,
    "palabras_totales": <numero>
  },
  "resumen": {
    "puntuacion_global": <0-100>,
    "nivel": "<excelente|bueno|aceptable|deficiente>",
    "descripcion": "<2 oraciones síntesis>",
    "fortaleza": "<1 fortaleza concreta>",
    "mejorar": "<1 área a mejorar>"
  },
  "dimensiones": {
    "claridad": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación 1 oración>"},
    "estructura": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación>"},
    "persuasion": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación>"},
    "proposito": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación>"},
    "empatia": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación>"},
    "adaptacion": {"puntaje": <0-100>, "nivel": "<bueno|regular|bajo>", "tu_resultado": "<observación>"}
  },
  "calidad_global": {
    "puntaje": <promedio dimensiones>,
    "nivel": "<excelente|bueno|aceptable|deficiente>"
  },
  "recomendaciones": [
    {"prioridad": 1, "categoria": "<dimensión>", "accion": "<acción concreta 10 palabras>"},
    {"prioridad": 2, "categoria": "<dimensión>", "accion": "<acción concreta>"},
    {"prioridad": 3, "categoria": "<dimensión>", "accion": "<acción concreta>"}
  ]
}

CALIBRACIÓN DE PUNTAJES (ESTRICTA, NO generoso):
- 0-15: desastroso (no logra comunicar)
- 16-40: deficiente (entiende pero falla en ejecutar)
- 41-60: aceptable (cumple básico, mejorable)
- 61-80: bueno (efectivo, pequeñas áreas mejorar)
- 81-95: excelente
- 96-100: perfecto (raro)

REGLAS DE EVALUACIÓN:
- Si culpa al cliente o no cierra acuerdos: persuasion/proposito ≤ 40
- Si tiene >5 muletillas/min: claridad ≤ 50
- Si no hay estructura clara apertura-desarrollo-cierre: estructura ≤ 50
- Si no se adapta al tono del interlocutor: adaptacion ≤ 50
- Empatía requiere validar emociones, no solo escuchar

Genera JSON COMPLETO. Brevedad > detalle. Cierra el JSON."#;
