//! Custom prompts persistidos en DB para que equipos enterprise personalicen
//! el coach sin tocar código. Activable por `purpose` (tips/eval/chat/prospecting).
//! Si no hay prompt activo, se usan los prompts hardcoded por defecto.

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub id: i64,
    pub name: String,
    pub purpose: String,
    pub prompt_text: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCustomPromptInput {
    pub name: String,
    pub purpose: String,
    pub prompt_text: String,
    pub activate: Option<bool>,
}

const VALID_PURPOSES: &[&str] = &["tips", "evaluation", "chat", "prospecting"];

fn validate_purpose(p: &str) -> Result<(), String> {
    if VALID_PURPOSES.contains(&p) {
        Ok(())
    } else {
        Err(format!(
            "purpose inválido: {} (válidos: {})",
            p,
            VALID_PURPOSES.join(",")
        ))
    }
}

pub async fn ensure_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS custom_prompts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            purpose TEXT NOT NULL,
            prompt_text TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
         )",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_custom_prompts_purpose_active
         ON custom_prompts(purpose, is_active)",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_active_prompt(
    pool: &SqlitePool,
    purpose: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT prompt_text FROM custom_prompts
         WHERE purpose = ? AND is_active = 1
         ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(purpose)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(p,)| p))
}

#[tauri::command]
pub async fn coach_save_custom_prompt(
    state: tauri::State<'_, AppState>,
    input: CreateCustomPromptInput,
) -> Result<i64, String> {
    validate_purpose(&input.purpose)?;
    if input.name.trim().is_empty() {
        return Err("Nombre vacío".to_string());
    }
    if input.prompt_text.trim().len() < 20 {
        return Err("Prompt demasiado corto (mínimo 20 caracteres)".to_string());
    }
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Init: {}", e))?;

    let activate = input.activate.unwrap_or(false);

    if activate {
        // Desactivar otros prompts del mismo purpose
        sqlx::query("UPDATE custom_prompts SET is_active = 0 WHERE purpose = ?")
            .bind(&input.purpose)
            .execute(pool)
            .await
            .map_err(|e| format!("Desactivar: {}", e))?;
    }

    let result = sqlx::query(
        "INSERT INTO custom_prompts (name, purpose, prompt_text, is_active)
         VALUES (?, ?, ?, ?)",
    )
    .bind(&input.name)
    .bind(&input.purpose)
    .bind(&input.prompt_text)
    .bind(if activate { 1 } else { 0 })
    .execute(pool)
    .await
    .map_err(|e| format!("Insert: {}", e))?;

    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn coach_list_custom_prompts(
    state: tauri::State<'_, AppState>,
    purpose: Option<String>,
) -> Result<Vec<CustomPrompt>, String> {
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Init: {}", e))?;

    let rows: Vec<(i64, String, String, String, i64, String, String)> = if let Some(p) = purpose {
        sqlx::query_as(
            "SELECT id, name, purpose, prompt_text, is_active, created_at, updated_at
             FROM custom_prompts WHERE purpose = ? ORDER BY updated_at DESC",
        )
        .bind(p)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("List: {}", e))?
    } else {
        sqlx::query_as(
            "SELECT id, name, purpose, prompt_text, is_active, created_at, updated_at
             FROM custom_prompts ORDER BY purpose, updated_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("List: {}", e))?
    };

    Ok(rows
        .into_iter()
        .map(|(id, name, purpose, prompt_text, is_active, created_at, updated_at)| CustomPrompt {
            id,
            name,
            purpose,
            prompt_text,
            is_active: is_active != 0,
            created_at,
            updated_at,
        })
        .collect())
}

#[tauri::command]
pub async fn coach_set_active_custom_prompt(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Init: {}", e))?;

    let purpose: Option<(String,)> =
        sqlx::query_as("SELECT purpose FROM custom_prompts WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Lookup: {}", e))?;

    let purpose = purpose.map(|(p,)| p).ok_or_else(|| format!("Prompt id {} no existe", id))?;

    sqlx::query("UPDATE custom_prompts SET is_active = 0 WHERE purpose = ?")
        .bind(&purpose)
        .execute(pool)
        .await
        .map_err(|e| format!("Reset: {}", e))?;

    sqlx::query("UPDATE custom_prompts SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Activate: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn coach_delete_custom_prompt(
    state: tauri::State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let pool = state.db_manager.pool();
    sqlx::query("DELETE FROM custom_prompts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Delete: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ensure_table_idempotent() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        ensure_table(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn save_and_retrieve_active() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO custom_prompts (name, purpose, prompt_text, is_active) VALUES (?, ?, ?, 1)",
        )
        .bind("Mi prompt enterprise")
        .bind("tips")
        .bind("Eres un coach experto en SaaS B2B...")
        .execute(&pool)
        .await
        .unwrap();
        let active = get_active_prompt(&pool, "tips").await.unwrap();
        assert!(active.is_some());
        assert!(active.unwrap().contains("SaaS B2B"));
    }

    #[tokio::test]
    async fn no_active_returns_none() {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ensure_table(&pool).await.unwrap();
        let active = get_active_prompt(&pool, "tips").await.unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn validate_purpose_accepts_known() {
        assert!(validate_purpose("tips").is_ok());
        assert!(validate_purpose("evaluation").is_ok());
        assert!(validate_purpose("chat").is_ok());
        assert!(validate_purpose("prospecting").is_ok());
        assert!(validate_purpose("invalid").is_err());
    }
}
