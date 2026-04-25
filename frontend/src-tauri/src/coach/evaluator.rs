//! Evaluador de comunicación post-llamada.
//!
//! Toma la transcripción completa al cerrar una reunión y devuelve métricas
//! 0-10 (clarity, engagement, structure, overall) más fortalezas y áreas
//! de mejora. Reusa `summary::llm_client::generate_summary`.
//!
//! Adoptado de `D:/Maity_Desktop/summary/communication_evaluator.rs` —
//! adaptado al proyecto: provider FIJO Ollama (privacidad), parser tolerante,
//! tests unitarios.

use crate::coach::evaluation_types::MeetingEvaluation;
use crate::coach::prompts::evaluation_v4::{EVALUATION_V4_SYSTEM_PROMPT, PROMPT_VERSION};
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;
use tauri::Manager;

/// Observaciones detalladas por categoría.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunicationObservations {
    pub clarity: Option<String>,
    pub structure: Option<String>,
    pub objections: Option<String>,
    pub calls_to_action: Option<String>,
}

/// Métricas de comunicación calculadas sin LLM (heurísticas sobre el transcript).
/// V3.1: complementan el scoring del LLM con datos duros.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunicationMetrics {
    /// Conteo total de muletillas del USER (eh, este, o sea, pues...).
    pub filler_count: usize,
    /// Muletillas por minuto de habla del USER.
    pub fillers_per_minute: f32,
    /// Preguntas abiertas del USER (qué/cómo/cuál/dónde/cuándo).
    pub open_questions_count: usize,
    /// Preguntas cerradas del USER (sí/no, verdad, cierto, no?).
    pub closed_questions_count: usize,
    /// Conteo de frases de validación del USER (entiendo, comprendo, veo que...).
    pub validations_given: usize,
    /// Veces que el USER habló con tono atropellado (turnos rapid_fire).
    pub rapid_fire_turns: usize,
    /// Promedio de palabras por turno del USER.
    pub avg_user_turn_words: f32,
    /// Talk ratio del USER (0.0-1.0).
    pub user_talk_ratio: f32,
}

/// Feedback completo de comunicación con scores 0-10 + análisis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommunicationFeedback {
    pub overall_score: Option<f32>,
    pub clarity: Option<f32>,
    pub engagement: Option<f32>,
    pub structure: Option<f32>,
    pub feedback: Option<String>,
    pub summary: Option<String>,
    pub strengths: Option<Vec<String>>,
    pub areas_to_improve: Option<Vec<String>>,
    pub observations: Option<CommunicationObservations>,
    /// V3.1: métricas objetivas calculadas localmente (sin LLM).
    pub metrics: Option<CommunicationMetrics>,
    /// Modelo y latencia para A/B testing
    pub model: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Calcula métricas objetivas de comunicación sobre la transcripción completa.
///
/// `user_text`: concatenación de todos los turnos del USER.
/// `interlocutor_text`: concatenación de todos los turnos del INTERLOCUTOR.
pub fn compute_metrics(user_text: &str, interlocutor_text: &str, user_turn_count: usize) -> CommunicationMetrics {
    let user_words = user_text.split_whitespace().count();
    let total_words = user_words + interlocutor_text.split_whitespace().count();

    let filler_count = crate::coach::trigger::count_filler_words(user_text);

    // Estimación de minutos de habla del USER: ~130 palabras/min en español conversacional.
    let user_speech_minutes = (user_words as f32 / 130.0).max(0.01);
    let fillers_per_minute = filler_count as f32 / user_speech_minutes;

    // Preguntas: contar '?' en user_text y clasificar por palabras cercanas.
    let (open_q, closed_q) = classify_questions(user_text);

    // Validaciones: contar frases empáticas.
    const VALIDATIONS: &[&str] = &[
        "entiendo", "comprendo", "veo que", "escucho", "tiene razon", "tienes razon",
        "es valido", "lamento", "siento que",
    ];
    let norm_user = user_text.to_lowercase();
    let validations_given = VALIDATIONS.iter().map(|v| norm_user.matches(v).count()).sum();

    // Rapid-fire: estimación — cuenta turnos con >50 palabras (aproximación).
    // No podemos ver los turnos individuales aquí, así que lo dejamos en 0 como fallback.
    let rapid_fire_turns = 0;

    let avg_user_turn_words = if user_turn_count > 0 {
        user_words as f32 / user_turn_count as f32
    } else {
        0.0
    };

    let user_talk_ratio = if total_words > 0 {
        user_words as f32 / total_words as f32
    } else {
        0.0
    };

    CommunicationMetrics {
        filler_count,
        fillers_per_minute,
        open_questions_count: open_q,
        closed_questions_count: closed_q,
        validations_given,
        rapid_fire_turns,
        avg_user_turn_words,
        user_talk_ratio,
    }
}

/// Clasifica preguntas del USER en abiertas (qué/cómo/cuál/dónde/cuándo/por qué) vs cerradas.
///
/// Segmenta por '?' y analiza si la oración empieza con un interrogativo abierto.
fn classify_questions(user_text: &str) -> (usize, usize) {
    const OPEN_STARTERS: &[&str] = &[
        "que ", "qué ", "como ", "cómo ", "cual ", "cuál ", "donde ", "dónde ",
        "cuando ", "cuándo ", "por que ", "por qué ", "para que ", "para qué ",
        "quien ", "quién ",
    ];
    let mut open = 0;
    let mut closed = 0;
    // Separar oraciones-pregunta cada vez que aparece '?'. Para cada una, mirar
    // los últimos ~40 chars previos al '?' y buscar el inicio de la oración
    // (tras '¿', '.', '!', '\n' previo).
    let bytes: Vec<char> = user_text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '?' {
            // Encontrar el inicio de la oración: backtrack a '¿', '.', '!' o '\n'.
            let mut start = i;
            while start > 0 {
                let c = bytes[start - 1];
                if c == '¿' || c == '.' || c == '!' || c == '\n' {
                    break;
                }
                start -= 1;
            }
            // Saltar espacios y '¿' iniciales.
            let slice: String = bytes[start..i]
                .iter()
                .skip_while(|c| c.is_whitespace() || **c == '¿')
                .collect();
            let lower = slice.to_lowercase();
            let is_open = OPEN_STARTERS.iter().any(|s| lower.starts_with(s));
            if is_open {
                open += 1;
            } else {
                closed += 1;
            }
        }
        i += 1;
    }
    (open, closed)
}

const EVALUATION_SYSTEM_PROMPT: &str = r#"Eres un coach de comunicación profesional. Analiza la transcripción de una reunión y evalúa las habilidades de comunicación del usuario (identificado como "USUARIO" o "user" — el hablante del micrófono).

MÉTRICAS (escala 0-10, decimales permitidos):
- clarity: qué tan claro y comprensible es el mensaje del usuario
- engagement: qué tan participativo e involucrado está
- structure: qué tan organizado es el discurso
- overall_score: puntuación general

REGLAS ESTRICTAS:
1. Responde ÚNICAMENTE con JSON válido, sin markdown, sin texto antes ni después.
2. Sé específico y constructivo. Cita comportamientos OBSERVABLES en la transcripción.
3. NO inventes datos. Si no hay suficiente material para una métrica, déjala en null.
4. strengths y areas_to_improve: 2-4 elementos cada uno, MAX 15 palabras cada uno.
5. feedback: 1-2 oraciones de resumen accionable.

Formato exacto:
{
  "overall_score": 7.5,
  "clarity": 8.0,
  "engagement": 7.0,
  "structure": 7.5,
  "feedback": "Resumen breve y accionable",
  "strengths": ["Fortaleza 1", "Fortaleza 2"],
  "areas_to_improve": ["Área 1", "Área 2"],
  "observations": {
    "clarity": "Observación específica",
    "structure": "Observación específica",
    "objections": "Cómo manejó objeciones",
    "calls_to_action": "Análisis de cierres y propuestas"
  }
}"#;

/// Evalúa la comunicación del usuario en una transcripción completa.
///
/// **Privacidad**: solo usa Ollama (igual que `coach_suggest`).
#[tauri::command]
pub async fn coach_evaluate_communication(
    transcript: String,
    model: Option<String>,
) -> Result<CommunicationFeedback, String> {
    if transcript.trim().len() < 50 {
        return Err("Transcripción demasiado corta para evaluar (min 50 caracteres)".to_string());
    }

    let model_to_use = model.unwrap_or_else(|| {
        crate::coach::prompt::DEFAULT_MODEL.to_string()
    });

    let user_prompt = format!(
        "Analiza la siguiente transcripción de reunión y evalúa al USUARIO:\n\n<transcripcion>\n{}\n</transcripcion>\n\nResponde SOLO con el JSON.",
        transcript
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let start = std::time::Instant::now();

    let raw = generate_summary(
        &client,
        &LLMProvider::Ollama,
        &model_to_use,
        "",
        EVALUATION_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(800), // suficiente para feedback completo
        Some(0.3), // baja temperatura: queremos análisis consistente
        Some(0.95),
        None,
        None,
    )
    .await
    .map_err(|e| format!("Error LLM: {}", e))?;

    let latency_ms = start.elapsed().as_millis() as u64;

    let mut feedback = parse_evaluation_response(&raw)?;
    feedback.model = Some(model_to_use);
    feedback.latency_ms = Some(latency_ms);

    Ok(feedback)
}

/// Parsea la respuesta del LLM. Tolerante a markdown wrapping y ruido.
fn parse_evaluation_response(response: &str) -> Result<CommunicationFeedback, String> {
    let json_str = extract_json_from_response(response);

    serde_json::from_str::<CommunicationFeedback>(&json_str)
        .map_err(|e| format!("JSON inválido del evaluador: {} | raw: {}", e, json_str))
}

/// Extrae el primer bloque JSON entre `{` y `}`.
fn extract_json_from_response(response: &str) -> String {
    let trimmed = response.trim();
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if start < end {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_simple() {
        let raw = r#"{"overall_score":7.5,"clarity":8.0}"#;
        assert_eq!(extract_json_from_response(raw), raw);
    }

    #[test]
    fn test_extract_json_con_ruido() {
        let raw = "Aquí va: {\"overall_score\":7.5} listo.";
        let json = extract_json_from_response(raw);
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("overall_score"));
    }

    #[test]
    fn test_extract_json_con_markdown() {
        let raw = "```json\n{\"overall_score\":7.5}\n```";
        let json = extract_json_from_response(raw);
        assert!(json.contains("overall_score"));
    }

    #[test]
    fn test_parse_feedback_completo() {
        let json = r#"{
            "overall_score": 7.5,
            "clarity": 8.0,
            "engagement": 7.0,
            "structure": 7.5,
            "feedback": "Buen ritmo y claridad",
            "strengths": ["Claro", "Empático"],
            "areas_to_improve": ["Cerrar con call to action"]
        }"#;
        let result = parse_evaluation_response(json).unwrap();
        assert_eq!(result.overall_score, Some(7.5));
        assert_eq!(result.clarity, Some(8.0));
        assert_eq!(result.strengths.unwrap().len(), 2);
    }

    #[test]
    fn test_parse_feedback_minimo() {
        // Todos los campos opcionales, solo overall_score
        let json = r#"{"overall_score":6.0}"#;
        let result = parse_evaluation_response(json).unwrap();
        assert_eq!(result.overall_score, Some(6.0));
        assert!(result.clarity.is_none());
    }

    #[test]
    fn test_parse_invalido() {
        assert!(parse_evaluation_response("texto sin json").is_err());
    }

    #[test]
    fn test_compute_metrics_fillers() {
        let user = "hola eh este o sea pues la reunion digamos va bien";
        let inter = "claro, perfecto";
        let m = compute_metrics(user, inter, 1);
        assert!(m.filler_count >= 4, "filler_count={}", m.filler_count);
        assert!(m.fillers_per_minute > 0.0);
        assert!(m.user_talk_ratio > 0.5);
    }

    #[test]
    fn test_compute_metrics_validations() {
        let user = "entiendo tu punto. comprendo la preocupación. veo que esto es importante.";
        let inter = "si exacto";
        let m = compute_metrics(user, inter, 3);
        assert_eq!(m.validations_given, 3);
    }

    #[test]
    fn test_compute_metrics_talk_ratio_bajo() {
        let user = "si claro";
        let inter = "tenemos un problema muy grande con el servicio y necesitamos una solucion rapida para poder avanzar con el proyecto";
        let m = compute_metrics(user, inter, 1);
        assert!(m.user_talk_ratio < 0.2, "ratio={}", m.user_talk_ratio);
    }

    #[test]
    fn test_classify_questions_abierta() {
        let (open, closed) = classify_questions("¿qué te parece? ¿cómo lo ves?");
        assert_eq!(open, 2);
        assert_eq!(closed, 0);
    }

    #[test]
    fn test_classify_questions_cerrada() {
        let (open, closed) = classify_questions("¿te parece bien? ¿estás de acuerdo?");
        assert_eq!(open, 0);
        assert!(closed >= 2);
    }

    #[test]
    fn test_compute_metrics_empty() {
        let m = compute_metrics("", "", 0);
        assert_eq!(m.filler_count, 0);
        assert_eq!(m.avg_user_turn_words, 0.0);
        assert_eq!(m.user_talk_ratio, 0.0);
    }
}

// ============================================================================
// EVALUACIÓN POST-MEETING v4 (Gemma 4) — JSON estructurado completo
// ============================================================================

/// Resultado persistido de evaluación post-meeting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMeetingEvaluationResult {
    pub meeting_id: String,
    pub evaluation: MeetingEvaluation,
    pub model_used: String,
    pub prompt_version: String,
    pub latency_ms: u64,
    pub created_at: String,
}

/// Modelo Gemma 4 sugerido por defecto. Configurable vía settings o argumento.
pub const DEFAULT_EVALUATION_MODEL: &str = "gemma3:4b";

/// Genera evaluación profunda post-meeting con Gemma 4 (~12k chars JSON).
/// Persiste en `meeting_evaluations` y devuelve el resultado completo.
#[tauri::command]
pub async fn coach_evaluate_post_meeting<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    meeting_id: String,
    transcript: String,
    previous_session_id: Option<String>,
    evaluation_model: Option<String>,
) -> Result<PostMeetingEvaluationResult, String> {
    if transcript.trim().len() < 100 {
        return Err("Transcripción demasiado corta para evaluación profunda (min 100 caracteres)".to_string());
    }

    let model = evaluation_model.unwrap_or_else(|| DEFAULT_EVALUATION_MODEL.to_string());

    let prev_score: Option<f32> = if let Some(prev_id) = previous_session_id.as_ref() {
        if let Some(state) = app.try_state::<AppState>() {
            let pool = state.db_manager.pool();
            sqlx::query_scalar::<_, Option<f32>>(
                "SELECT puntuacion_global FROM meeting_evaluations WHERE meeting_id = ?",
            )
            .bind(prev_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .flatten()
        } else {
            None
        }
    } else {
        None
    };

    let user_prompt = format!(
        "Analiza la siguiente transcripción de reunión.\n\n\
         meeting_id: {}\n\
         sesion_anterior_id: {}\n\
         puntuacion_anterior: {}\n\n\
         <transcripcion>\n{}\n</transcripcion>\n\n\
         Responde SOLO con el JSON completo según la estructura definida.",
        meeting_id,
        previous_session_id.as_deref().unwrap_or("null"),
        prev_score
            .map(|s| format!("{:.1}", s))
            .unwrap_or_else(|| "null".to_string()),
        transcript
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;

    let start = std::time::Instant::now();

    let raw = generate_summary(
        &client,
        &LLMProvider::Ollama,
        &model,
        "",
        EVALUATION_V4_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(4096),
        Some(0.2),
        Some(0.9),
        None,
        None,
    )
    .await
    .map_err(|e| format!("Error LLM (¿modelo {} no instalado? `ollama pull {}`): {}", model, model, e))?;

    let latency_ms = start.elapsed().as_millis() as u64;

    let json_str = extract_json_from_response(&raw);
    let mut evaluation: MeetingEvaluation = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON inválido del evaluador v4: {} | raw len={}", e, raw.len()))?;

    if evaluation.identificacion.sesion_id.is_none() {
        evaluation.identificacion.sesion_id = Some(meeting_id.clone());
    }
    if evaluation.identificacion.version_prompt.is_empty() {
        evaluation.identificacion.version_prompt = PROMPT_VERSION.to_string();
    }
    if let Some(prev_id) = previous_session_id.as_ref() {
        evaluation.historico.sesion_anterior_id = Some(prev_id.clone());
        if let Some(prev) = prev_score {
            let delta = evaluation.resumen.puntuacion_global - prev;
            evaluation.historico.tendencia_global = Some(delta);
        }
    }

    if let Some(state) = app.try_state::<AppState>() {
        let pool = state.db_manager.pool();
        if let Err(e) = persist_evaluation(pool, &meeting_id, &evaluation, &model, previous_session_id.as_deref()).await {
            log::warn!("No se pudo persistir evaluación: {}", e);
        }
    }

    Ok(PostMeetingEvaluationResult {
        meeting_id,
        evaluation,
        model_used: model,
        prompt_version: PROMPT_VERSION.to_string(),
        latency_ms,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn persist_evaluation(
    pool: &SqlitePool,
    meeting_id: &str,
    evaluation: &MeetingEvaluation,
    model_used: &str,
    sesion_anterior_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    let json_str = serde_json::to_string(evaluation)
        .unwrap_or_else(|_| "{}".to_string());
    let nivel = if evaluation.resumen.nivel.is_empty() {
        evaluation.calidad_global.nivel.clone()
    } else {
        evaluation.resumen.nivel.clone()
    };
    let puntuacion = if evaluation.resumen.puntuacion_global > 0.0 {
        evaluation.resumen.puntuacion_global
    } else {
        evaluation.calidad_global.puntaje
    };

    sqlx::query(
        "INSERT INTO meeting_evaluations
            (meeting_id, evaluation_json, model_used, prompt_version,
             puntuacion_global, nivel, duration_minutes, sesion_anterior_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(meeting_id) DO UPDATE SET
            evaluation_json = excluded.evaluation_json,
            model_used = excluded.model_used,
            prompt_version = excluded.prompt_version,
            puntuacion_global = excluded.puntuacion_global,
            nivel = excluded.nivel,
            duration_minutes = excluded.duration_minutes,
            sesion_anterior_id = excluded.sesion_anterior_id,
            created_at = CURRENT_TIMESTAMP",
    )
    .bind(meeting_id)
    .bind(json_str)
    .bind(model_used)
    .bind(PROMPT_VERSION)
    .bind(puntuacion as f64)
    .bind(nivel)
    .bind(evaluation.meta.duracion_minutos as i64)
    .bind(sesion_anterior_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Recupera evaluación previamente persistida (None si no existe aún).
#[tauri::command]
pub async fn coach_get_post_meeting_evaluation(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Option<PostMeetingEvaluationResult>, String> {
    let pool = state.db_manager.pool();
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT evaluation_json, model_used, prompt_version, created_at
         FROM meeting_evaluations WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    if let Some((json_str, model_used, prompt_version, created_at)) = row {
        let evaluation: MeetingEvaluation = serde_json::from_str(&json_str)
            .map_err(|e| format!("Evaluación corrupta en DB: {}", e))?;
        Ok(Some(PostMeetingEvaluationResult {
            meeting_id,
            evaluation,
            model_used,
            prompt_version,
            latency_ms: 0,
            created_at,
        }))
    } else {
        Ok(None)
    }
}
