//! Chat conversacional sobre una reunión específica usando búsqueda semántica.
//!
//! Pipeline:
//! 1. Embedding de la pregunta con `nomic-embed-text` (Ollama local).
//! 2. Búsqueda top_k segmentos relevantes filtrados por `meeting_id` en
//!    `transcript_embeddings`.
//! 3. Construye prompt con segmentos como contexto + cita literal con timestamp.
//! 4. Genera respuesta con Gemma 4 (configurable) — provider FIJO Ollama.
//!
//! Diferencia respecto a `coach_chat`: usa segmentos *relevantes* en vez de
//! ventana reciente, ideal para preguntas sobre reuniones cerradas.

use crate::coach::commands::SHARED_CLIENT;
use crate::semantic_search::{cosine_similarity, repository::EmbeddingsRepository, DEFAULT_EMBED_MODEL};
use crate::semantic_search::embedder::embed_text;
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use serde::{Deserialize, Serialize};
use tauri::Manager;

const DEFAULT_CHAT_MODEL: &str = "gemma3:4b";
const DEFAULT_TOP_K: usize = 5;

const MEETING_CHAT_SYSTEM_PROMPT: &str = r#"Eres un asistente que responde preguntas sobre una reunión específica que ya ocurrió. Tienes acceso a fragmentos relevantes del transcript con timestamps.

REGLAS:
1. Responde SOLO basándote en lo que aparece en los fragmentos. No inventes datos.
2. Cita literalmente cuando sea relevante con formato [MM:SS] al lado de la cita.
3. Sé directo y conciso (máximo 4 oraciones, salvo que se pida detalle).
4. Si los fragmentos no contienen la respuesta, dilo: "No encontré evidencia en la reunión sobre eso."
5. Diferencia speakers: USUARIO es el dueño del micrófono, INTERLOCUTOR es la otra persona.
6. Tono profesional, español neutro, sin preámbulos."#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingChatCitation {
    pub segment_id: String,
    pub text: String,
    pub source_type: Option<String>,
    pub audio_start_time: Option<f64>,
    pub audio_end_time: Option<f64>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingChatResponse {
    pub answer: String,
    pub citations: Vec<MeetingChatCitation>,
    pub model: String,
    pub latency_ms: u64,
    pub matched_segments: usize,
}

fn format_timestamp(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}

#[tauri::command]
pub async fn chat_with_meeting(
    app: tauri::AppHandle,
    meeting_id: String,
    query: String,
    top_k: Option<u32>,
    chat_model: Option<String>,
    embed_model: Option<String>,
) -> Result<MeetingChatResponse, String> {
    if query.trim().is_empty() {
        return Err("La pregunta no puede estar vacía".to_string());
    }
    if meeting_id.trim().is_empty() {
        return Err("meeting_id requerido".to_string());
    }

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let embed_model = embed_model.unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
    let chat_model = chat_model.unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());
    let k = top_k.unwrap_or(DEFAULT_TOP_K as u32).max(1).min(20) as usize;

    let client = &*SHARED_CLIENT;

    let query_emb = embed_text(client, &embed_model, &query, None)
        .await
        .map_err(|e| {
            format!(
                "Error embedding query (¿modelo {} no instalado? `ollama pull {}`): {}",
                embed_model, embed_model, e
            )
        })?;

    let rows = EmbeddingsRepository::load_all(pool, &embed_model, Some(&meeting_id))
        .await
        .map_err(|e| format!("Error cargando embeddings: {}", e))?;

    if rows.is_empty() {
        return Ok(MeetingChatResponse {
            answer: "Esta reunión aún no tiene embeddings indexados. Genera el índice antes de chatear.".to_string(),
            citations: vec![],
            model: chat_model,
            latency_ms: 0,
            matched_segments: 0,
        });
    }

    let mut scored: Vec<(f32, _)> = rows
        .into_iter()
        .map(|r| (cosine_similarity(&query_emb, &r.embedding), r))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let top: Vec<_> = scored.into_iter().take(k).collect();

    let mut context_block = String::new();
    let mut citations: Vec<MeetingChatCitation> = Vec::with_capacity(top.len());
    for (score, row) in top.iter() {
        let speaker_label = match row.source_type.as_deref() {
            Some("user") => "USUARIO",
            Some("interlocutor") => "INTERLOCUTOR",
            _ => "DESCONOCIDO",
        };
        let ts_label = row
            .audio_start_time
            .map(format_timestamp)
            .unwrap_or_else(|| "??:??".to_string());
        context_block.push_str(&format!(
            "[{}] {} ({}): {}\n",
            ts_label, speaker_label, row.segment_id, row.text
        ));
        citations.push(MeetingChatCitation {
            segment_id: row.segment_id.clone(),
            text: row.text.clone(),
            source_type: row.source_type.clone(),
            audio_start_time: row.audio_start_time,
            audio_end_time: row.audio_end_time,
            score: *score,
        });
    }

    let user_prompt = format!(
        "<fragmentos_relevantes meeting=\"{}\" total=\"{}\">\n{}\n</fragmentos_relevantes>\n\nPregunta:\n{}",
        meeting_id,
        citations.len(),
        context_block,
        query.trim()
    );

    let start = std::time::Instant::now();
    let raw = generate_summary(
        client,
        &LLMProvider::Ollama,
        &chat_model,
        "",
        MEETING_CHAT_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(800),
        Some(0.4),
        Some(0.9),
        None,
        None,
    )
    .await
    .map_err(|e| {
        format!(
            "Error LLM chat (¿modelo {} no instalado? `ollama pull {}`): {}",
            chat_model, chat_model, e
        )
    })?;
    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(MeetingChatResponse {
        answer: raw.trim().to_string(),
        citations,
        model: chat_model,
        latency_ms,
        matched_segments: top.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn timestamp_formats_seconds() {
        assert_eq!(format_timestamp(0.0), "00:00");
        assert_eq!(format_timestamp(65.4), "01:05");
        assert_eq!(format_timestamp(3725.9), "62:05");
    }

    #[test]
    fn timestamp_clamps_negative() {
        assert_eq!(format_timestamp(-3.0), "00:00");
    }
}
