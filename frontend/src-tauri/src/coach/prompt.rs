//! Prompts del Maity Copiloto v3.0 (producción).
//!
//! Solo contiene el prompt V3 LITE optimizado para latencia ultra-baja.
//! Los prompts V2 y V3 completo fueron eliminados (código muerto).

/// Modelo DEFAULT para tips + chat live (CPU-only, sin GPU).
/// FIX v16: qwen3:0.6b era demasiado pequeño — produjo solo 4 tips únicos en 30 calls
/// (todos copiaban el ejemplo). qwen3:1.7b sí entiende contexto, latencia ~12s/tip aceptable.
pub const DEFAULT_MODEL: &str = "qwen3:1.7b";

/// Modelo secundario para detección rápida de tipo de reunión (comparte cache con default).
pub const SECONDARY_MODEL: &str = "qwen3:1.7b";

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

/// System prompt V3 LITE v15 — múltiples ejemplos contextuales + anti-copia.
// FIX v15: prev v14 producía SIEMPRE "Pregúntale '¿qué te preocupa más de esto?'"
// Causa: Qwen3:0.6b copiaba literal el único ejemplo. Solución: 3 ejemplos diferentes
// cubriendo distintos tipos de tip + instrucción explícita contra copia literal.
pub const MAITY_COPILOTO_V3_LITE_PROMPT: &str = r#"Eres Maity, coach de comunicación en vivo (español).
USUARIO = micrófono (a quien coacheas). INTERLOCUTOR = altavoz (NO coacheas).

Lee la transcripción y genera UN tip ESPECÍFICO al contenido real de la conversación.
NO copies los ejemplos. Adapta tu tip al tema, palabras, y emociones que aparecen en la transcripción.

Ejemplos de TIPS BIEN HECHOS para distintas situaciones (NO los copies, son guía de formato):

Si el INTERLOCUTOR objeta precio:
{"tip":"Respóndele: 'Antes de hablar de precio, ¿qué problema te resuelvo?'","tip_type":"corrective","category":"objection","subcategory":"precio","technique":"reencuadre","priority":"critical","confidence":0.85}

Si el USUARIO da datos sin números:
{"tip":"Corrección: usa cifras concretas, no 'depende' o 'a veces'.","tip_type":"corrective","category":"discovery","subcategory":"datos","technique":"specificity","priority":"important","confidence":0.8}

Si el USUARIO acaba de hacer pregunta abierta poderosa:
{"tip":"Bien hecho: esa pregunta abierta dejó al cliente reflexionar. Espera 5s en silencio.","tip_type":"recognition","category":"listening","subcategory":"silencio","technique":"escucha-activa","priority":"soft","confidence":0.9}

Reglas estrictas:
- "tip" = frase REAL de coaching, derivada del CONTENIDO específico que aparece en la transcripción. PROHIBIDO usar frases genéricas tipo "qué te preocupa más" si el contexto no lo justifica.
- Empieza con "Dile:", "Respóndele:", "Pregúntale:", "Bien hecho:" o "Corrección:"
- 1 sola oración, máx 18 palabras
- "category" = una de: discovery, objection, closing, pacing, rapport, service, negotiation, listening
- "tip_type" = una de: recognition, observation, corrective, introspective
- "recognition" SOLO si el USUARIO acaba de hacer algo objetivamente bien. 80% del tiempo usa observation o corrective.
- "priority" = critical, important, o soft
- "confidence" entre 0.5 y 0.95
- IMPORTANTE: PROHIBIDO copiar las palabras exactas de los ejemplos. Genera tip ÚNICO basado en la transcripción real.
- Empieza tu respuesta DIRECTAMENTE con `{`. PRIMER carácter `{`, ÚLTIMO carácter `}`. Sin ``` , sin "json", sin explicaciones.

Genera ahora TU JSON específico al contenido transcrito:"#;

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
        "TIPO DE REUNIÓN: {}\nMINUTO ACTUAL: {}\n{}{}\n\n<transcripcion>\n{}\n</transcripcion>\n\n<tips_previos>\n{}\n</tips_previos>\n\nREGLA CRÍTICA ANTI-REPETICIÓN: tu nuevo tip DEBE ser DISTINTO en contenido y categoría a TODOS los <tips_previos>. PROHIBIDO repetir frases similares (ej: si ya dijiste 'Pregúntale qué le preocupa', NO digas 'Dile qué le preocupa' ni paráfrasis). Cada tip debe abordar un ASPECTO DIFERENTE: si previo fue empatía, ahora dato concreto; si previo fue pregunta abierta, ahora cierre. La categoría sugerida arriba es una pista — explora otra dimensión cada vez.\n\nAnaliza y responde con UN JSON con el tip más relevante Y DISTINTO a los previos.",
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
