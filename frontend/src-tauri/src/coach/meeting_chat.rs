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
use crate::semantic_search::DEFAULT_EMBED_MODEL;
use crate::semantic_search::embedder::embed_text;
use crate::semantic_search::search::streaming_top_k;
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
    /// Identificador y título de la reunión donde proviene el fragmento.
    /// Solo presente en chat global (`chat_with_history`); en chat por reunión
    /// (`chat_with_meeting`) el meeting es ya implícito.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_title: Option<String>,
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
    let chat_model = chat_model
        .or_else(|| {
            crate::coach::commands::CHAT_MODEL
                .lock()
                .ok()
                .map(|m| m.clone())
        })
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());
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

    let scored = streaming_top_k(pool, &embed_model, Some(&meeting_id), &query_emb, k)
        .await
        .map_err(|e| format!("Error buscando embeddings: {}", e))?;

    if scored.is_empty() {
        return Ok(MeetingChatResponse {
            answer: "Esta reunión aún no tiene embeddings indexados. Genera el índice antes de chatear.".to_string(),
            citations: vec![],
            model: chat_model,
            latency_ms: 0,
            matched_segments: 0,
        });
    }

    let top: Vec<(f32, crate::semantic_search::repository::EmbeddingRow)> =
        scored.into_iter().map(|s| (s.score, s.row)).collect();

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
            meeting_id: None,
            meeting_title: None,
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

const GLOBAL_CHAT_SYSTEM_PROMPT: &str = r#"Eres un asistente que responde preguntas sobre el HISTORIAL COMPLETO de reuniones del usuario. Tienes acceso a fragmentos relevantes de DIVERSAS reuniones, cada uno con título, fecha y timestamp.

REGLAS:
1. Responde SOLO basándote en lo que aparece en los fragmentos. No inventes datos.
2. Cuando cites, identifica la reunión: usa el formato [Título reunión, MM:SS].
3. Si la pregunta abarca varias reuniones, sintetiza patrones (ej: "en 3 de 5 reuniones surgió X").
4. Si los fragmentos no contienen la respuesta, dilo: "No encontré evidencia en tu historial sobre eso."
5. Sé conciso (máximo 5 oraciones, salvo que se pida detalle).
6. Diferencia speakers: USUARIO eres tú, INTERLOCUTOR es la otra persona.
7. Tono profesional, español neutro, sin preámbulos."#;

/// Chat global: busca embeddings sobre TODAS las reuniones indexadas y responde
/// con citas multi-reunión. Útil para preguntas como "¿qué objeciones recurrentes
/// han surgido este mes?" o "¿cuáles son mis acuerdos pendientes?".
#[tauri::command]
pub async fn chat_with_history(
    app: tauri::AppHandle,
    query: String,
    top_k: Option<u32>,
    chat_model: Option<String>,
    embed_model: Option<String>,
) -> Result<MeetingChatResponse, String> {
    if query.trim().is_empty() {
        return Err("La pregunta no puede estar vacía".to_string());
    }

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let embed_model = embed_model.unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
    let chat_model = chat_model
        .or_else(|| {
            crate::coach::commands::CHAT_MODEL
                .lock()
                .ok()
                .map(|m| m.clone())
        })
        .unwrap_or_else(|| DEFAULT_CHAT_MODEL.to_string());
    let k = top_k.unwrap_or(8).max(1).min(30) as usize;

    let client = &*SHARED_CLIENT;

    let query_emb = embed_text(client, &embed_model, &query, None)
        .await
        .map_err(|e| {
            format!(
                "Error embedding query (¿modelo {} no instalado? `ollama pull {}`): {}",
                embed_model, embed_model, e
            )
        })?;

    let scored = streaming_top_k(pool, &embed_model, None, &query_emb, k)
        .await
        .map_err(|e| format!("Error buscando embeddings: {}", e))?;

    if scored.is_empty() {
        return Ok(MeetingChatResponse {
            answer: "No hay reuniones indexadas todavía. Graba algunas reuniones para empezar a chatear con tu historial.".to_string(),
            citations: vec![],
            model: chat_model,
            latency_ms: 0,
            matched_segments: 0,
        });
    }

    let top: Vec<(f32, crate::semantic_search::repository::EmbeddingRow)> =
        scored.into_iter().map(|s| (s.score, s.row)).collect();

    // Cargar títulos de meetings citados (lookup masivo) para evitar N consultas.
    let meeting_ids: std::collections::HashSet<String> =
        top.iter().map(|(_, r)| r.meeting_id.clone()).collect();
    let mut titles: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if !meeting_ids.is_empty() {
        let placeholders = vec!["?"; meeting_ids.len()].join(",");
        let sql = format!("SELECT id, title FROM meetings WHERE id IN ({})", placeholders);
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for id in &meeting_ids {
            q = q.bind(id);
        }
        if let Ok(rows) = q.fetch_all(pool).await {
            for (id, title) in rows {
                titles.insert(id, title);
            }
        }
    }

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
        let title = titles
            .get(&row.meeting_id)
            .cloned()
            .unwrap_or_else(|| row.meeting_id.clone());
        context_block.push_str(&format!(
            "[{}, {}] {}: {}\n",
            title, ts_label, speaker_label, row.text
        ));
        citations.push(MeetingChatCitation {
            segment_id: row.segment_id.clone(),
            text: row.text.clone(),
            source_type: row.source_type.clone(),
            audio_start_time: row.audio_start_time,
            audio_end_time: row.audio_end_time,
            score: *score,
            meeting_id: Some(row.meeting_id.clone()),
            meeting_title: Some(title),
        });
    }

    let user_prompt = format!(
        "<fragmentos_relevantes total=\"{}\">\n{}\n</fragmentos_relevantes>\n\nPregunta:\n{}",
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
        GLOBAL_CHAT_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(900),
        Some(0.4),
        Some(0.9),
        None,
        None,
    )
    .await
    .map_err(|e| {
        format!(
            "Error LLM chat global (¿modelo {} no instalado? `ollama pull {}`): {}",
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
