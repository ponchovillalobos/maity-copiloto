//! CRUD de la tabla `dev_iterations` + endpoints Tauri para el dashboard.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// Datos para INSERT al terminar una iteración (`/dev` audio import).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewIterationRecord {
    pub meeting_id: String,
    pub iteration_label: Option<String>,
    pub audio_user_path: Option<String>,
    pub audio_interlocutor_path: Option<String>,
    pub channel_layout: String,
    pub total_duration_seconds: f64,
    pub decode_ms: Option<i64>,
    pub transcribe_user_ms: Option<i64>,
    pub transcribe_interlocutor_ms: Option<i64>,
    pub evaluation_ms: Option<i64>,
    pub total_pipeline_ms: Option<i64>,
    pub wer_global: Option<f32>,
    pub wer_user: Option<f32>,
    pub wer_interlocutor: Option<f32>,
    pub hypothesis_full: Option<String>,
    pub reference_user: Option<String>,
    pub reference_interlocutor: Option<String>,
    pub evaluation_score: Option<f32>,
    pub evaluation_sections_filled: Option<i64>,
    pub prompt_version: String,
    pub coach_model: String,
    pub evaluation_model: String,
    pub cpu_avg_pct: Option<f32>,
    pub ram_peak_mb: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationRow {
    pub id: i64,
    pub meeting_id: String,
    pub iteration_label: Option<String>,
    pub channel_layout: String,
    pub total_duration_seconds: f64,
    pub decode_ms: Option<i64>,
    pub transcribe_user_ms: Option<i64>,
    pub transcribe_interlocutor_ms: Option<i64>,
    pub evaluation_ms: Option<i64>,
    pub total_pipeline_ms: Option<i64>,
    pub wer_global: Option<f32>,
    pub wer_user: Option<f32>,
    pub wer_interlocutor: Option<f32>,
    pub evaluation_score: Option<f32>,
    pub evaluation_sections_filled: Option<i64>,
    pub prompt_version: String,
    pub coach_model: String,
    pub evaluation_model: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationDetail {
    pub row: IterationRow,
    pub hypothesis_full: Option<String>,
    pub reference_user: Option<String>,
    pub reference_interlocutor: Option<String>,
    pub audio_user_path: Option<String>,
    pub audio_interlocutor_path: Option<String>,
    pub cpu_avg_pct: Option<f32>,
    pub ram_peak_mb: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_iterations: i64,
    pub iterations_last_7d: i64,
    pub avg_wer_user_30d: Option<f32>,
    pub avg_wer_interlocutor_30d: Option<f32>,
    pub avg_evaluation_score_30d: Option<f32>,
    pub avg_total_pipeline_ms_30d: Option<f32>,
    pub last_iteration_at: Option<String>,
    pub broken_button_count: i64,
    pub untested_button_count: i64,
}

fn row_to_iteration(r: &sqlx::sqlite::SqliteRow) -> IterationRow {
    IterationRow {
        id: r.get("id"),
        meeting_id: r.get("meeting_id"),
        iteration_label: r.try_get("iteration_label").ok(),
        channel_layout: r.get("channel_layout"),
        total_duration_seconds: r.get("total_duration_seconds"),
        decode_ms: r.try_get("decode_ms").ok(),
        transcribe_user_ms: r.try_get("transcribe_user_ms").ok(),
        transcribe_interlocutor_ms: r.try_get("transcribe_interlocutor_ms").ok(),
        evaluation_ms: r.try_get("evaluation_ms").ok(),
        total_pipeline_ms: r.try_get("total_pipeline_ms").ok(),
        wer_global: r.try_get("wer_global").ok(),
        wer_user: r.try_get("wer_user").ok(),
        wer_interlocutor: r.try_get("wer_interlocutor").ok(),
        evaluation_score: r.try_get("evaluation_score").ok(),
        evaluation_sections_filled: r.try_get("evaluation_sections_filled").ok(),
        prompt_version: r.get("prompt_version"),
        coach_model: r.get("coach_model"),
        evaluation_model: r.get("evaluation_model"),
        created_at: r.get("created_at"),
    }
}

pub async fn insert_iteration(
    pool: &SqlitePool,
    rec: &NewIterationRecord,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO dev_iterations (
            meeting_id, iteration_label,
            audio_user_path, audio_interlocutor_path, channel_layout,
            total_duration_seconds,
            decode_ms, transcribe_user_ms, transcribe_interlocutor_ms,
            evaluation_ms, total_pipeline_ms,
            wer_global, wer_user, wer_interlocutor,
            hypothesis_full, reference_user, reference_interlocutor,
            evaluation_score, evaluation_sections_filled,
            prompt_version, coach_model, evaluation_model,
            cpu_avg_pct, ram_peak_mb, notes
         ) VALUES (
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?,
            ?, ?,
            ?, ?, ?,
            ?, ?, ?
         )",
    )
    .bind(&rec.meeting_id)
    .bind(&rec.iteration_label)
    .bind(&rec.audio_user_path)
    .bind(&rec.audio_interlocutor_path)
    .bind(&rec.channel_layout)
    .bind(rec.total_duration_seconds)
    .bind(rec.decode_ms)
    .bind(rec.transcribe_user_ms)
    .bind(rec.transcribe_interlocutor_ms)
    .bind(rec.evaluation_ms)
    .bind(rec.total_pipeline_ms)
    .bind(rec.wer_global)
    .bind(rec.wer_user)
    .bind(rec.wer_interlocutor)
    .bind(&rec.hypothesis_full)
    .bind(&rec.reference_user)
    .bind(&rec.reference_interlocutor)
    .bind(rec.evaluation_score)
    .bind(rec.evaluation_sections_filled)
    .bind(&rec.prompt_version)
    .bind(&rec.coach_model)
    .bind(&rec.evaluation_model)
    .bind(rec.cpu_avg_pct)
    .bind(rec.ram_peak_mb)
    .bind(&rec.notes)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

#[tauri::command]
pub async fn dashboard_list_iterations(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<IterationRow>, String> {
    let pool = state.db_manager.pool();
    let limit = limit.unwrap_or(100).min(500) as i64;
    let rows = sqlx::query(
        "SELECT id, meeting_id, iteration_label, channel_layout, total_duration_seconds,
                decode_ms, transcribe_user_ms, transcribe_interlocutor_ms, evaluation_ms, total_pipeline_ms,
                wer_global, wer_user, wer_interlocutor,
                evaluation_score, evaluation_sections_filled,
                prompt_version, coach_model, evaluation_model, created_at
         FROM dev_iterations
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    Ok(rows.iter().map(row_to_iteration).collect())
}

#[tauri::command]
pub async fn dashboard_get_iteration_detail(
    state: tauri::State<'_, AppState>,
    iteration_id: i64,
) -> Result<Option<IterationDetail>, String> {
    let pool = state.db_manager.pool();
    let row = sqlx::query(
        "SELECT id, meeting_id, iteration_label, channel_layout, total_duration_seconds,
                decode_ms, transcribe_user_ms, transcribe_interlocutor_ms, evaluation_ms, total_pipeline_ms,
                wer_global, wer_user, wer_interlocutor,
                evaluation_score, evaluation_sections_filled,
                prompt_version, coach_model, evaluation_model, created_at,
                hypothesis_full, reference_user, reference_interlocutor,
                audio_user_path, audio_interlocutor_path,
                cpu_avg_pct, ram_peak_mb, notes
         FROM dev_iterations
         WHERE id = ?",
    )
    .bind(iteration_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    Ok(row.map(|r| IterationDetail {
        row: row_to_iteration(&r),
        hypothesis_full: r.try_get("hypothesis_full").ok(),
        reference_user: r.try_get("reference_user").ok(),
        reference_interlocutor: r.try_get("reference_interlocutor").ok(),
        audio_user_path: r.try_get("audio_user_path").ok(),
        audio_interlocutor_path: r.try_get("audio_interlocutor_path").ok(),
        cpu_avg_pct: r.try_get("cpu_avg_pct").ok(),
        ram_peak_mb: r.try_get("ram_peak_mb").ok(),
        notes: r.try_get("notes").ok(),
    }))
}

#[tauri::command]
pub async fn dashboard_get_summary(
    state: tauri::State<'_, AppState>,
) -> Result<DashboardSummary, String> {
    let pool = state.db_manager.pool();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dev_iterations")
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    let last_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dev_iterations WHERE created_at >= datetime('now', '-7 days')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let avg_wer_user: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(wer_user) FROM dev_iterations
         WHERE wer_user IS NOT NULL AND created_at >= datetime('now', '-30 days')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let avg_wer_inter: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(wer_interlocutor) FROM dev_iterations
         WHERE wer_interlocutor IS NOT NULL AND created_at >= datetime('now', '-30 days')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let avg_score: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(evaluation_score) FROM dev_iterations
         WHERE evaluation_score IS NOT NULL AND created_at >= datetime('now', '-30 days')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let avg_total_ms: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(total_pipeline_ms) FROM dev_iterations
         WHERE total_pipeline_ms IS NOT NULL AND created_at >= datetime('now', '-30 days')",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    let last_at: Option<String> =
        sqlx::query_scalar("SELECT MAX(created_at) FROM dev_iterations")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    let broken_button_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM button_audit WHERE status = 'broken'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    let untested_button_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM button_audit WHERE status = 'untested'")
            .fetch_one(pool)
            .await
            .map_err(|e| format!("DB error: {}", e))?;

    Ok(DashboardSummary {
        total_iterations: total,
        iterations_last_7d: last_7d,
        avg_wer_user_30d: avg_wer_user.map(|x| x as f32),
        avg_wer_interlocutor_30d: avg_wer_inter.map(|x| x as f32),
        avg_evaluation_score_30d: avg_score.map(|x| x as f32),
        avg_total_pipeline_ms_30d: avg_total_ms.map(|x| x as f32),
        last_iteration_at: last_at,
        broken_button_count,
        untested_button_count,
    })
}
