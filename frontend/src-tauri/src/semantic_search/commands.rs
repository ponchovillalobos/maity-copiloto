//! Comandos Tauri de búsqueda semántica.
//!
//! - `semantic_index_meeting(meeting_id)`: indexa todos los segmentos
//!   de una reunión usando Ollama embeddings.
//! - `semantic_search(query, top_k, meeting_id?)`: busca por similitud coseno
//!   sobre embeddings ya indexados.
//! - `semantic_get_index_stats(meeting_id?)`: cuenta segmentos indexados.

use std::sync::LazyLock;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::embedder::embed_text;
#[allow(unused_imports)]
use super::repository::EmbeddingsRepository;
use super::{IndexResult, SearchResult, DEFAULT_EMBED_MODEL};
use crate::state::AppState;

pub static EMBED_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .pool_max_idle_per_host(2)
        .build()
        .expect("failed to build embed http client")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_segments: u32,
    pub indexed_segments: u32,
    pub model: String,
}

#[tauri::command]
pub async fn semantic_index_meeting(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    model: Option<String>,
    endpoint: Option<String>,
) -> Result<IndexResult, String> {
    let start = Instant::now();
    let model = model.unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
    let pool = state.db_manager.pool();

    let rows = sqlx::query(
        "SELECT id, transcript, audio_start_time, audio_end_time, speaker
         FROM transcripts WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error fetching transcripts: {}", e))?;

    if rows.is_empty() {
        return Err(format!("No transcripts for meeting {}", meeting_id));
    }

    let mut indexed = 0_u32;
    let mut skipped = 0_u32;

    for row in rows {
        let segment_id: String = row.try_get("id").map_err(|e| e.to_string())?;
        let text: String = row.try_get("transcript").map_err(|e| e.to_string())?;
        let start_t: Option<f64> = row.try_get("audio_start_time").ok();
        let end_t: Option<f64> = row.try_get("audio_end_time").ok();
        let speaker: Option<String> = row.try_get("speaker").ok();

        if text.trim().is_empty() {
            skipped += 1;
            continue;
        }

        if EmbeddingsRepository::segment_already_indexed(pool, &meeting_id, &segment_id, &model)
            .await
            .map_err(|e| e.to_string())?
        {
            skipped += 1;
            continue;
        }

        let emb = match embed_text(&EMBED_HTTP, &model, &text, endpoint.as_deref()).await {
            Ok(v) => v,
            Err(e) => {
                return Err(format!(
                    "Embed failed at segment {} ({}/{} OK before): {}",
                    segment_id, indexed, indexed + skipped, e
                ));
            }
        };

        EmbeddingsRepository::upsert(
            pool,
            &meeting_id,
            &segment_id,
            &text,
            &emb,
            &model,
            start_t,
            end_t,
            speaker.as_deref(),
        )
        .await
        .map_err(|e| format!("DB upsert failed: {}", e))?;

        indexed += 1;
    }

    Ok(IndexResult {
        meeting_id,
        indexed_count: indexed,
        skipped_count: skipped,
        model,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn semantic_search(
    state: tauri::State<'_, AppState>,
    query: String,
    top_k: Option<u32>,
    meeting_id: Option<String>,
    model: Option<String>,
    endpoint: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let top_k = top_k.unwrap_or(10).max(1).min(100) as usize;
    let model = model.unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
    let pool = state.db_manager.pool();

    let query_emb = embed_text(&EMBED_HTTP, &model, &query, endpoint.as_deref())
        .await
        .map_err(|e| format!("Embed query failed: {}", e))?;

    let scored = super::search::streaming_top_k(
        pool,
        &model,
        meeting_id.as_deref(),
        &query_emb,
        top_k,
    )
    .await
    .map_err(|e| format!("DB search failed: {}", e))?;

    Ok(scored
        .into_iter()
        .map(|s| SearchResult {
            meeting_id: s.row.meeting_id,
            segment_id: s.row.segment_id,
            text: s.row.text,
            score: s.score,
            audio_start_time: s.row.audio_start_time,
            audio_end_time: s.row.audio_end_time,
            source_type: s.row.source_type,
        })
        .collect())
}

#[tauri::command]
pub async fn semantic_get_index_stats(
    state: tauri::State<'_, AppState>,
    meeting_id: Option<String>,
    model: Option<String>,
) -> Result<IndexStats, String> {
    let model = model.unwrap_or_else(|| DEFAULT_EMBED_MODEL.to_string());
    let pool = state.db_manager.pool();

    let total: i64 = if let Some(mid) = &meeting_id {
        sqlx::query_scalar("SELECT COUNT(*) FROM transcripts WHERE meeting_id = ?")
            .bind(mid)
            .fetch_one(pool)
            .await
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM transcripts")
            .fetch_one(pool)
            .await
    }
    .map_err(|e| e.to_string())?;

    let indexed: i64 = if let Some(mid) = &meeting_id {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM transcript_embeddings WHERE meeting_id = ? AND model = ?",
        )
        .bind(mid)
        .bind(&model)
        .fetch_one(pool)
        .await
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM transcript_embeddings WHERE model = ?")
            .bind(&model)
            .fetch_one(pool)
            .await
    }
    .map_err(|e| e.to_string())?;

    Ok(IndexStats {
        total_segments: total as u32,
        indexed_segments: indexed as u32,
        model,
    })
}
