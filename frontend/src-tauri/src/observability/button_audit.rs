//! CRUD + seed para la matriz de auditoría de botones (`button_audit`).

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonRow {
    pub id: String,
    pub display_name: String,
    pub source_file: String,
    pub source_line: Option<i64>,
    pub category: String,
    pub status: String,
    pub notes: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_checked_iteration_id: Option<i64>,
}

/// Lista predefinida de los botones críticos. Idempotente: si ya existen, no
/// los toca. Si se agregan nuevos botones al código, agregar entradas aquí.
fn seed_definitions() -> Vec<ButtonRow> {
    let entries = [
        // Recording
        ("rec.start", "Iniciar grabación", "frontend/src/components/RecordingControls.tsx", 89, "recording"),
        ("rec.stop", "Detener grabación", "frontend/src/components/RecordingControls.tsx", 0, "recording"),
        ("rec.pause", "Pausar/Reanudar", "frontend/src/components/RecordingControls.tsx", 0, "recording"),
        // Sidebar / Quick actions
        ("sidebar.new_meeting", "Nueva reunión", "frontend/src/components/Sidebar/QuickActions.tsx", 14, "navigation"),
        ("sidebar.search", "Buscar", "frontend/src/components/Sidebar/QuickActions.tsx", 14, "navigation"),
        ("sidebar.toggle_coach", "Toggle Coach", "frontend/src/components/Sidebar/QuickActions.tsx", 14, "coach"),
        ("sidebar.export_pdf", "Export PDF", "frontend/src/components/Sidebar/QuickActions.tsx", 48, "export"),
        ("sidebar.delete_meeting", "Eliminar reunión", "frontend/src/components/Sidebar/index.tsx", 0, "navigation"),
        // Command palette
        ("cmd.new_recording", "Nueva grabación (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.export_pdf", "Exportar PDF (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.export_md", "Exportar Markdown (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.export_json", "Exportar JSON (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.semantic_search", "Búsqueda semántica (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.global_chat", "Chat global (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.open_floating", "Abrir flotante (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        ("cmd.dashboard", "Dashboard (Ctrl+K)", "frontend/src/components/CommandPalette.tsx", 16, "command"),
        // Coach
        ("coach.request_tip", "Pedir tip ahora", "frontend/src/app/floating/page.tsx", 0, "coach"),
        ("coach.tip_next", "Tip siguiente", "frontend/src/app/floating/page.tsx", 0, "coach"),
        ("coach.tip_prev", "Tip anterior", "frontend/src/app/floating/page.tsx", 0, "coach"),
        // Evaluation
        ("eval.generate", "Generar evaluación", "frontend/src/components/MeetingEvaluation/EvaluationPanel.tsx", 0, "evaluation"),
        ("eval.export_pdf", "Exportar evaluación PDF", "frontend/src/components/MeetingEvaluation/EvaluationPanel.tsx", 0, "evaluation"),
        ("eval.compliance_report", "Compliance report", "frontend/src/components/MeetingEvaluation/EvaluationPanel.tsx", 0, "evaluation"),
        // Summary
        ("summary.generate", "Generar resumen", "frontend/src/components/MeetingDetails/SummaryPanel.tsx", 0, "summary"),
        ("summary.template_select", "Seleccionar plantilla", "frontend/src/components/MeetingDetails/SummaryGeneratorButtonGroup.tsx", 0, "summary"),
        // Dev
        ("dev.import_audio", "Procesar audio simulado", "frontend/src/app/dev/page.tsx", 0, "dev"),
        ("dev.qa_two_audios", "Procesar QA con WER", "frontend/src/app/dev/page.tsx", 0, "dev"),
    ];

    entries
        .iter()
        .map(|(id, name, file, line, cat)| ButtonRow {
            id: id.to_string(),
            display_name: name.to_string(),
            source_file: file.to_string(),
            source_line: if *line == 0 { None } else { Some(*line) },
            category: cat.to_string(),
            status: "untested".to_string(),
            notes: None,
            last_checked_at: None,
            last_checked_iteration_id: None,
        })
        .collect()
}

pub async fn seed_buttons_if_needed(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let mut inserted = 0usize;
    for b in seed_definitions() {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM button_audit WHERE id = ?")
            .bind(&b.id)
            .fetch_one(pool)
            .await?;
        if exists == 0 {
            sqlx::query(
                "INSERT INTO button_audit (id, display_name, source_file, source_line, category, status)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(&b.id)
            .bind(&b.display_name)
            .bind(&b.source_file)
            .bind(b.source_line)
            .bind(&b.category)
            .bind(&b.status)
            .execute(pool)
            .await?;
            inserted += 1;
        }
    }
    Ok(inserted)
}

#[tauri::command]
pub async fn dashboard_seed_buttons(
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    seed_buttons_if_needed(state.db_manager.pool())
        .await
        .map_err(|e| format!("DB error: {}", e))
}

#[tauri::command]
pub async fn dashboard_list_buttons(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ButtonRow>, String> {
    let pool = state.db_manager.pool();
    let rows = sqlx::query(
        "SELECT id, display_name, source_file, source_line, category, status,
                notes, last_checked_at, last_checked_iteration_id
         FROM button_audit
         ORDER BY category, display_name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB error: {}", e))?;

    Ok(rows
        .iter()
        .map(|r| ButtonRow {
            id: r.get("id"),
            display_name: r.get("display_name"),
            source_file: r.get("source_file"),
            source_line: r.try_get("source_line").ok(),
            category: r.get("category"),
            status: r.get("status"),
            notes: r.try_get("notes").ok(),
            last_checked_at: r.try_get("last_checked_at").ok(),
            last_checked_iteration_id: r.try_get("last_checked_iteration_id").ok(),
        })
        .collect())
}

#[tauri::command]
pub async fn dashboard_update_button_status(
    state: tauri::State<'_, AppState>,
    button_id: String,
    status: String,
    notes: Option<String>,
    iteration_id: Option<i64>,
) -> Result<(), String> {
    let valid = ["ok", "broken", "warn", "untested", "deprecated"];
    if !valid.contains(&status.as_str()) {
        return Err(format!("Status inválido: {}", status));
    }
    sqlx::query(
        "UPDATE button_audit
         SET status = ?, notes = ?, last_checked_at = CURRENT_TIMESTAMP, last_checked_iteration_id = ?
         WHERE id = ?",
    )
    .bind(&status)
    .bind(&notes)
    .bind(iteration_id)
    .bind(&button_id)
    .execute(state.db_manager.pool())
    .await
    .map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}
