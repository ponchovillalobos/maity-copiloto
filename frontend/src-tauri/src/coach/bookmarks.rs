//! Bookmarks para reuniones — captura de momentos importantes.
//!
//! Permite marcar timestamps con categorías y notas para referencia
//! rápida durante/después de reuniones.

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Categorías válidas de bookmarks.
const VALID_CATEGORIES: &[&str] = &[
    "important",
    "follow_up",
    "pricing",
    "decision",
    "action_item",
    "risk",
];

/// Estructura de un bookmark (manual query mapping, no FromRow).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub recording_id: String,
    pub timestamp_sec: f64,
    pub category: String,
    pub note: Option<String>,
    pub segment_text: Option<String>,
    pub created_at: String,
}

/// Agrega un bookmark a la base de datos.
#[tauri::command]
pub async fn coach_add_bookmark(
    app: tauri::AppHandle,
    recording_id: String,
    timestamp_sec: f64,
    category: String,
    note: Option<String>,
    segment_text: Option<String>,
) -> Result<Bookmark, String> {
    if recording_id.trim().is_empty() {
        return Err("recording_id no puede estar vacío".to_string());
    }
    if timestamp_sec < 0.0 {
        return Err("timestamp_sec no puede ser negativo".to_string());
    }
    if !VALID_CATEGORIES.contains(&category.as_str()) {
        return Err(format!(
            "category inválida: {}. Válidas: {}",
            category,
            VALID_CATEGORIES.join(", ")
        ));
    }

    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let row = sqlx::query(
        "INSERT INTO bookmarks (recording_id, timestamp_sec, category, note, segment_text, created_at)
         VALUES (?, ?, ?, ?, ?, datetime('now'))
         RETURNING id, recording_id, timestamp_sec, category, note, segment_text, created_at"
    )
    .bind(&recording_id)
    .bind(timestamp_sec)
    .bind(&category)
    .bind(&note)
    .bind(&segment_text)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Error insertando bookmark: {}", e))?;

    use sqlx::Row;
    Ok(Bookmark {
        id: row.get("id"),
        recording_id: row.get("recording_id"),
        timestamp_sec: row.get("timestamp_sec"),
        category: row.get("category"),
        note: row.get("note"),
        segment_text: row.get("segment_text"),
        created_at: row.get("created_at"),
    })
}

/// Obtiene todos los bookmarks de una reunión ordenados por timestamp.
#[tauri::command]
pub async fn coach_get_bookmarks(
    app: tauri::AppHandle,
    recording_id: String,
) -> Result<Vec<Bookmark>, String> {
    if recording_id.trim().is_empty() {
        return Err("recording_id no puede estar vacío".to_string());
    }

    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let rows = sqlx::query(
        "SELECT id, recording_id, timestamp_sec, category, note, segment_text, created_at
         FROM bookmarks WHERE recording_id = ? ORDER BY timestamp_sec ASC"
    )
    .bind(&recording_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Error obteniendo bookmarks: {}", e))?;

    use sqlx::Row;
    let bookmarks = rows
        .iter()
        .map(|row| Bookmark {
            id: row.get("id"),
            recording_id: row.get("recording_id"),
            timestamp_sec: row.get("timestamp_sec"),
            category: row.get("category"),
            note: row.get("note"),
            segment_text: row.get("segment_text"),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(bookmarks)
}

/// Elimina un bookmark por ID.
#[tauri::command]
pub async fn coach_delete_bookmark(
    app: tauri::AppHandle,
    id: i64,
) -> Result<(), String> {
    if id <= 0 {
        return Err("ID debe ser positivo".to_string());
    }

    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let result = sqlx::query("DELETE FROM bookmarks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Error eliminando bookmark: {}", e))?;

    if result.rows_affected() == 0 {
        return Err(format!("Bookmark con ID {} no encontrado", id));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_serializes() {
        let b = Bookmark {
            id: 1,
            recording_id: "meeting-123".into(),
            timestamp_sec: 45.5,
            category: "important".into(),
            note: Some("Revisar".into()),
            segment_text: Some("Cliente mencionó presupuesto".into()),
            created_at: "2026-04-24T10:30:00".into(),
        };
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("meeting-123"));
        assert!(json.contains("45.5"));
    }

    #[test]
    fn test_valid_categories() {
        for cat in VALID_CATEGORIES {
            assert!(VALID_CATEGORIES.contains(cat));
        }
        assert!(!VALID_CATEGORIES.contains(&"invalid"));
    }

    #[test]
    fn test_bookmark_optional_fields() {
        let b = Bookmark {
            id: 1,
            recording_id: "test".into(),
            timestamp_sec: 10.0,
            category: "decision".into(),
            note: None,
            segment_text: None,
            created_at: String::new(),
        };
        assert!(b.note.is_none());
        assert!(b.segment_text.is_none());
    }
}
