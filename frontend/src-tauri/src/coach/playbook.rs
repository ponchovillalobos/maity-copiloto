//! Cross-prospect playbook: análisis de patrones a través de TODAS las reuniones.
//!
//! Pipeline:
//! 1. Embedding de la query del usuario (nomic-embed-text)
//! 2. Streaming top-k=15 sobre TODOS los embeddings (sin filtro meeting_id)
//! 3. Group por meeting_id, junta título + fecha + segmentos relevantes
//! 4. LLM gemma3:4b sintetiza patrón cross-meeting con citas
//! 5. Devuelve PlaybookInsight con recomendaciones accionables

use crate::coach::commands::SHARED_CLIENT;
use crate::semantic_search::{embedder::embed_text, search::streaming_top_k, DEFAULT_EMBED_MODEL};
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::Manager;

const PLAYBOOK_SYSTEM_PROMPT: &str = r#"Eres un coach de ventas analizando PATRONES a través de varias reuniones del usuario. Recibes fragmentos relevantes de DIVERSAS reuniones que matchean la query del usuario.

Tu tarea: identificar patrones recurrentes y construir un playbook accionable.

REGLAS:
1. Responde SOLO con JSON válido (sin markdown, sin texto extra antes o después).
2. Sintetiza el patrón general (ej: "en 4 de 6 reuniones, el cliente objeta el precio antes del minuto 10").
3. Para cada patrón, incluye 2-4 citas literales con `meeting_title` y `quote`.
4. Recomendaciones DEBEN ser concretas y accionables (no genéricas como "mejora rapport").
5. Si responses_exitosas tiene clones (USUARIO respondió bien antes), inclúyelas como "scripts validados".

Esquema JSON exacto:
{
  "patron_principal": "string (1-2 oraciones)",
  "frecuencia": "string (ej: '4 de 6 reuniones', '60% de los casos')",
  "citas_clave": [
    {"meeting_title": "string", "quote": "string", "speaker": "USUARIO|INTERLOCUTOR"}
  ],
  "scripts_validados": [
    {"contexto": "string", "respuesta_recomendada": "string", "fuente_meeting": "string"}
  ],
  "recomendaciones": ["string"],
  "anti_patrones": ["string (qué NO hacer basado en intentos fallidos)"]
}"#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybookCita {
    pub meeting_title: String,
    pub quote: String,
    pub speaker: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybookScript {
    pub contexto: String,
    pub respuesta_recomendada: String,
    pub fuente_meeting: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybookInsight {
    pub patron_principal: String,
    pub frecuencia: String,
    pub citas_clave: Vec<PlaybookCita>,
    pub scripts_validados: Vec<PlaybookScript>,
    pub recomendaciones: Vec<String>,
    pub anti_patrones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookResult {
    pub query: String,
    pub insight: PlaybookInsight,
    pub meetings_analyzed: usize,
    pub model: String,
    pub latency_ms: u64,
}

fn extract_json_block(raw: &str) -> &str {
    let s = raw.trim();
    if let Some(start) = s.find('{') {
        if let Some(end) = s.rfind('}') {
            if end > start {
                return &s[start..=end];
            }
        }
    }
    s
}

async fn fetch_meeting_titles(
    pool: &SqlitePool,
    ids: &std::collections::HashSet<String>,
) -> HashMap<String, String> {
    let mut titles: HashMap<String, String> = HashMap::new();
    if ids.is_empty() {
        return titles;
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("SELECT id, title FROM meetings WHERE id IN ({})", placeholders);
    let mut q = sqlx::query_as::<_, (String, String)>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    if let Ok(rows) = q.fetch_all(pool).await {
        for (id, title) in rows {
            titles.insert(id, title);
        }
    }
    titles
}

#[tauri::command]
pub async fn generate_playbook(
    app: tauri::AppHandle,
    query: String,
    top_k: Option<u32>,
    chat_model: Option<String>,
    embed_model: Option<String>,
) -> Result<PlaybookResult, String> {
    if query.trim().is_empty() {
        return Err("La query no puede estar vacía".to_string());
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
        .unwrap_or_else(|| "gemma3:4b".to_string());
    let k = top_k.unwrap_or(15).max(1).min(50) as usize;

    let client = &*SHARED_CLIENT;
    let query_emb = embed_text(client, &embed_model, &query, None)
        .await
        .map_err(|e| format!("Embed query failed: {}", e))?;

    let scored = streaming_top_k(pool, &embed_model, None, &query_emb, k)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;

    if scored.is_empty() {
        return Ok(PlaybookResult {
            query,
            insight: PlaybookInsight::default(),
            meetings_analyzed: 0,
            model: chat_model,
            latency_ms: 0,
        });
    }

    let meeting_ids: std::collections::HashSet<String> =
        scored.iter().map(|s| s.row.meeting_id.clone()).collect();
    let meetings_analyzed = meeting_ids.len();
    let titles = fetch_meeting_titles(pool, &meeting_ids).await;

    let mut context_block = String::new();
    for s in &scored {
        let speaker = match s.row.source_type.as_deref() {
            Some("user") => "USUARIO",
            Some("interlocutor") => "INTERLOCUTOR",
            _ => "DESCONOCIDO",
        };
        let title = titles
            .get(&s.row.meeting_id)
            .cloned()
            .unwrap_or_else(|| s.row.meeting_id.clone());
        context_block.push_str(&format!(
            "[{}] {}: {}\n",
            title, speaker, s.row.text
        ));
    }

    let user_prompt = format!(
        "<query>{}</query>\n\n<fragmentos total=\"{}\" meetings=\"{}\">\n{}\n</fragmentos>\n\nGenera el JSON playbook.",
        query.trim(),
        scored.len(),
        meetings_analyzed,
        context_block
    );

    let start = std::time::Instant::now();
    let raw = generate_summary(
        client,
        &LLMProvider::Ollama,
        &chat_model,
        "",
        PLAYBOOK_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(2048),
        Some(0.3),
        Some(0.9),
        None,
        None,
    )
    .await
    .map_err(|e| format!("Error LLM: {}", e))?;

    let latency_ms = start.elapsed().as_millis() as u64;
    let json_str = extract_json_block(&raw);
    let insight: PlaybookInsight = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON inválido: {}", e))?;

    Ok(PlaybookResult {
        query,
        insight,
        meetings_analyzed,
        model: chat_model,
        latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_playbook_json() {
        let raw = r#"{
            "patron_principal": "Cliente objeta precio antes del minuto 10",
            "frecuencia": "4 de 6",
            "citas_clave": [{"meeting_title":"X","quote":"es caro","speaker":"INTERLOCUTOR"}],
            "scripts_validados": [],
            "recomendaciones": ["Anticipar precio"],
            "anti_patrones": ["No defenderse"]
        }"#;
        let p: PlaybookInsight = serde_json::from_str(raw).unwrap();
        assert_eq!(p.frecuencia, "4 de 6");
        assert_eq!(p.citas_clave.len(), 1);
    }

    #[test]
    fn json_extraction_handles_prose() {
        let raw = "Aquí está: {\"patron_principal\":\"x\"} fin";
        assert!(extract_json_block(raw).starts_with('{'));
    }
}
