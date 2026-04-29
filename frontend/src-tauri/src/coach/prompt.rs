//! Prompts del Maity Copiloto v3.0 (producción).
//!
//! Solo contiene el prompt V3 LITE optimizado para latencia ultra-baja.
//! Los prompts V2 y V3 completo fueron eliminados (código muerto).

/// Modelo Ollama por defecto para tips + chat.
// Tips + chat usan Qwen 2.5 1.5B Q4 (1 GB, ~30 tok/s CPU).
// Antes probamos 0.5B pero copiaba el JSON schema literal en lugar de rellenar.
// 1.5B sí sigue instrucciones JSON correctamente.
// Evaluación post-meeting (calidad superior) sigue usando gemma3:4b.
pub const DEFAULT_MODEL: &str = "gemma3:1b";

/// Modelo secundario para detección rápida de tipo de reunión.
/// Antes era gemma3:4b (2.8 GB) — cambiado a gemma3:1b (380 MB) para
/// evitar cargar 2 modelos en RAM. Detección de tipo es 1 palabra, calidad sobra.
pub const SECONDARY_MODEL: &str = "gemma3:1b";

/// Tipos de reunión soportados por el copiloto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingType {
    Sales,
    Service,
    Webinar,
    TeamMeeting,
    Auto,
}

impl MeetingType {
    pub fn as_label(&self) -> &'static str {
        match self {
            MeetingType::Sales => "VENTA (discovery + cierre + objeciones)",
            MeetingType::Service => "SERVICIO AL CLIENTE (empatía + resolución)",
            MeetingType::Webinar => "WEBINAR / PRESENTACIÓN (pacing + engagement)",
            MeetingType::TeamMeeting => "REUNIÓN DE EQUIPO (facilitación + decisiones)",
            MeetingType::Auto => "REUNIÓN GENERAL",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "sales" | "venta" | "ventas" => MeetingType::Sales,
            "service" | "servicio" => MeetingType::Service,
            "webinar" | "presentacion" | "presentación" => MeetingType::Webinar,
            "team" | "team_meeting" | "equipo" | "junta" => MeetingType::TeamMeeting,
            _ => MeetingType::Auto,
        }
    }
}

/// System prompt V3 LITE — ~600 tokens, optimizado para tips accionables.
// PROMPT ULTRA-COMPACTO con ejemplo CONCRETO (Qwen 0.5B copiaba schema literal).
// La forma actual da 1 ejemplo JSON completo y rellenado, pidiendo replicar la estructura.
pub const MAITY_COPILOTO_V3_LITE_PROMPT: &str = r#"Eres Maity, coach de comunicación en vivo (español).
USUARIO = micrófono (a quien coacheas). INTERLOCUTOR = altavoz (NO coacheas).

Lee la transcripción y genera UN tip que el USUARIO debe DECIR AHORA.
Incluye la frase exacta entre comillas simples.

Ejemplo de tip CORRECTO (formato a replicar exactamente):
{"tip":"Pregúntale: '¿qué te preocupa más de esto?'","tip_type":"observation","category":"rapport","subcategory":"empatia","technique":"escucha-activa","priority":"important","confidence":0.85}

Reglas:
- "tip" = frase real de coaching, no descripción genérica
- Empieza con "Dile:", "Respóndele:", "Pregúntale:", "Bien hecho:" o "Corrección:"
- 1 sola oración, máx 15 palabras
- "category" = una de: discovery, objection, closing, pacing, rapport, service, negotiation, listening
- "tip_type" = una de: recognition, observation, corrective, introspective
- IMPORTANTE: usa "recognition" (felicitar) SOLO si el USUARIO acaba de hacer algo objetivamente bien (ej: pregunta abierta concreta, escucha activa visible). En el 80% de los casos elige observation o corrective. NO felicites por hablar normal.
- "priority" = critical, important, o soft
- IMPORTANTE: Empieza tu respuesta DIRECTAMENTE con el carácter `{`. No escribas ```, no escribas "json", no escribas "Aquí está", no escribas explicaciones. Tu PRIMER carácter debe ser `{` y tu ÚLTIMO carácter debe ser `}`.

Genera ahora el JSON con TU tip basado en la transcripción:"#;

/// Construye el user prompt v3.0 con toda la metadata.
pub fn build_user_prompt_v3(
    transcript: &str,
    meeting_type: MeetingType,
    minute: u32,
    previous_tips: &[String],
    suggested_category: Option<&str>,
    trigger_signal: Option<&str>,
) -> String {
    let previous_block = if previous_tips.is_empty() {
        String::from("(sin tips previos en esta sesión)")
    } else {
        previous_tips
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let category_hint = suggested_category
        .map(|c| format!("\nCATEGORÍA SUGERIDA POR TRIGGER: {} (usa como pista)", c))
        .unwrap_or_default();

    // Contexto de speaker: indica al LLM quién disparó el trigger
    let speaker_context = match trigger_signal {
        Some(sig) if sig.starts_with("client_") || sig.starts_with("interlocutor_") => {
            format!("\nSEÑAL DETECTADA: {} — disparada por INTERLOCUTOR. Tu tip va dirigido al USUARIO sobre cómo responder al interlocutor.", sig)
        }
        Some(sig) if sig.starts_with("user_") => {
            format!("\nSEÑAL DETECTADA: {} — disparada por USUARIO (micrófono). Tu tip debe corregir/guiar al USUARIO sobre SU propio comportamiento.", sig)
        }
        Some(sig) if sig.contains("last_speaker_interlocutor") => {
            "\nCHEQUEO PERIÓDICO. Último turno fue del INTERLOCUTOR. Analiza qué dijo el INTERLOCUTOR y sugiere al USUARIO cómo responder. NO confundas: lo que dijo el INTERLOCUTOR NO es culpa del USUARIO.".to_string()
        }
        Some(sig) if sig.contains("last_speaker_user") => {
            "\nCHEQUEO PERIÓDICO. Último turno fue del USUARIO. Evalúa cómo se comunicó el USUARIO y sugiere mejora sobre SU técnica.".to_string()
        }
        Some(sig) => {
            format!("\nSEÑAL DETECTADA: {}", sig)
        }
        None => {
            "\nCHEQUEO GENERAL. Lee la transcripción con atención: las líneas USUARIO: son del micrófono (a quien coacheas). Las líneas INTERLOCUTOR: son de la bocina (el otro). NO atribuyas al USUARIO lo que dijo el INTERLOCUTOR.".to_string()
        }
    };

    format!(
        "TIPO DE REUNIÓN: {}\nMINUTO ACTUAL: {}\n{}{}\n\n<transcripcion>\n{}\n</transcripcion>\n\n<tips_previos>\n{}\n</tips_previos>\n\nAnaliza y responde con UN JSON con el tip más relevante.",
        meeting_type.as_label(),
        minute,
        category_hint,
        speaker_context,
        transcript,
        previous_block
    )
}

/// Prompt corto para detectar el tipo de reunión con gemma3:4b.
pub const MEETING_TYPE_DETECTOR_PROMPT: &str = r#"Eres un clasificador de reuniones. Lees un fragmento de transcripción y devuelves SOLO UNA palabra con el tipo de reunión.

Opciones (responde exactamente una):
- sales        → venta, demo de producto, cotización, negociación comercial
- service      → servicio al cliente, soporte técnico, queja, reclamo
- webinar      → presentación, webinar, charla, monólogo de un speaker
- team_meeting → reunión de equipo, standup, retro, brainstorming
- auto         → no puedes determinar

RESPONDE SOLO UNA PALABRA. Sin explicaciones, sin JSON, sin markdown."#;

/// User prompt para el detector de tipo de reunión.
pub fn build_meeting_type_detector_prompt(transcript: &str) -> String {
    let preview: String = transcript.chars().take(1500).collect();
    format!(
        "Fragmento de conversación:\n\n{}\n\n¿Qué tipo de reunión es? Responde con UNA palabra.",
        preview
    )
}
