//! Tabla audit_log para registrar eventos compliance:
//!
//! CREATE TABLE IF NOT EXISTS audit_log (
//!     id INTEGER PRIMARY KEY AUTOINCREMENT,
//!     meeting_id TEXT,
//!     event_type TEXT NOT NULL,    -- llm_call | embed_call | transcript_save | recording_start | recording_stop | export
//!     endpoint TEXT,                -- ej "http://localhost:11434"
//!     model TEXT,                   -- ej "gemma3:4b"
//!     metadata TEXT,                -- JSON arbitrario
//!     timestamp INTEGER NOT NULL    -- unix ms
//! );

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Option<i64>,
    pub meeting_id: Option<String>,
    pub event_type: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub metadata: Option<String>,
    pub timestamp: i64,
}

pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            meeting_id TEXT,
            event_type TEXT NOT NULL,
            endpoint TEXT,
            model TEXT,
            metadata TEXT,
            timestamp INTEGER NOT NULL
         )",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_meeting ON audit_log(meeting_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_event(
    pool: &SqlitePool,
    meeting_id: Option<&str>,
    event_type: &str,
    endpoint: Option<&str>,
    model: Option<&str>,
    metadata: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let result = sqlx::query(
        "INSERT INTO audit_log (meeting_id, event_type, endpoint, model, metadata, timestamp)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(meeting_id)
    .bind(event_type)
    .bind(endpoint)
    .bind(model)
    .bind(metadata)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn get_events_for_meeting(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<AuditEvent>, sqlx::Error> {
    let rows: Vec<(i64, Option<String>, String, Option<String>, Option<String>, Option<String>, i64)> =
        sqlx::query_as(
            "SELECT id, meeting_id, event_type, endpoint, model, metadata, timestamp
             FROM audit_log
             WHERE meeting_id = ?
             ORDER BY timestamp ASC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|(id, mid, et, ep, m, meta, ts)| AuditEvent {
            id: Some(id),
            meeting_id: mid,
            event_type: et,
            endpoint: ep,
            model: m,
            metadata: meta,
            timestamp: ts,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_retrieve_audit_event() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        insert_event(
            &pool,
            Some("m1"),
            "llm_call",
            Some("http://localhost:11434"),
            Some("gemma3:4b"),
            Some(r#"{"tokens":150}"#),
        )
        .await
        .unwrap();
        let events = get_events_for_meeting(&pool, "m1").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "llm_call");
        assert_eq!(events[0].endpoint.as_deref(), Some("http://localhost:11434"));
    }

    #[tokio::test]
    async fn empty_when_no_events() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        let events = get_events_for_meeting(&pool, "missing").await.unwrap();
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn multiple_events_ordered() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        insert_event(&pool, Some("m1"), "start", None, None, None)
            .await
            .unwrap();
        insert_event(&pool, Some("m1"), "llm_call", Some("http://localhost:11434"), Some("phi"), None)
            .await
            .unwrap();
        let events = get_events_for_meeting(&pool, "m1").await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "start");
        assert_eq!(events[1].event_type, "llm_call");
    }
}
