//! Tauri commands compliance.

use crate::state::AppState;
use chrono::Utc;
use printpdf::{Mm, PdfDocument, BuiltinFont};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use tauri::Manager;

use super::audit_log::{insert_event, ensure_table};
use super::report::build_compliance_data;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceEvent {
    pub event_type: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub timestamp: i64,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMeetingAudit {
    pub meeting_id: String,
    pub transcript_hash: String,
    pub transcript_chars: usize,
    pub event_count: usize,
    pub external_endpoints_detected: Vec<String>,
    pub local_endpoints_used: Vec<String>,
    pub events: Vec<ComplianceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceExportResult {
    pub path: String,
    pub bytes: usize,
}

#[tauri::command]
pub async fn compliance_log_event(
    state: tauri::State<'_, AppState>,
    meeting_id: Option<String>,
    event_type: String,
    endpoint: Option<String>,
    model: Option<String>,
    metadata: Option<String>,
) -> Result<i64, String> {
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Audit init: {}", e))?;
    insert_event(pool, meeting_id.as_deref(), &event_type, endpoint.as_deref(), model.as_deref(), metadata.as_deref())
        .await
        .map_err(|e| format!("Audit insert: {}", e))
}

#[tauri::command]
pub async fn compliance_get_meeting_audit(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<ComplianceMeetingAudit, String> {
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Audit init: {}", e))?;
    let data = build_compliance_data(pool, &meeting_id).await?;
    Ok(ComplianceMeetingAudit {
        meeting_id: data.meeting_id,
        transcript_hash: data.transcript_hash,
        transcript_chars: data.transcript_chars,
        event_count: data.events.len(),
        external_endpoints_detected: data.external_endpoints_detected,
        local_endpoints_used: data.local_endpoints_used,
        events: data
            .events
            .into_iter()
            .map(|e| ComplianceEvent {
                event_type: e.event_type,
                endpoint: e.endpoint,
                model: e.model,
                timestamp: e.timestamp,
                metadata: e.metadata,
            })
            .collect(),
    })
}

fn default_export_path(meeting_id: &str) -> Result<PathBuf, String> {
    let base = dirs::data_dir()
        .ok_or_else(|| "No se encontró APPDATA".to_string())?
        .join("maity-desktop")
        .join("compliance-reports");
    std::fs::create_dir_all(&base).map_err(|e| format!("mkdir: {}", e))?;
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    Ok(base.join(format!("compliance_{}_{}.pdf", meeting_id, ts)))
}

#[tauri::command]
pub async fn compliance_export_report<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    meeting_id: String,
    output_path: Option<String>,
) -> Result<ComplianceExportResult, String> {
    let state = app.state::<AppState>();
    let pool = state.db_manager.pool();
    ensure_table(pool).await.map_err(|e| format!("Audit init: {}", e))?;
    let data = build_compliance_data(pool, &meeting_id).await?;

    let final_path = match output_path {
        Some(p) => PathBuf::from(p),
        None => default_export_path(&meeting_id)?,
    };

    let (doc, page1, layer1) =
        PdfDocument::new("Maity Compliance Report", Mm(210.0), Mm(297.0), "Layer1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| format!("font: {}", e))?;
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| format!("font: {}", e))?;
    let mono = doc.add_builtin_font(BuiltinFont::Courier).map_err(|e| format!("font: {}", e))?;

    let layer = doc.get_page(page1).get_layer(layer1);
    let mut y = 280.0;

    layer.use_text("MAITY COMPLIANCE REPORT", 18.0, Mm(20.0), Mm(y), &bold);
    y -= 8.0;
    layer.use_text(
        format!("Meeting: {}", data.meeting_id),
        10.0,
        Mm(20.0),
        Mm(y),
        &font,
    );
    y -= 5.0;
    layer.use_text(
        format!("Generado: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")),
        10.0,
        Mm(20.0),
        Mm(y),
        &font,
    );
    y -= 12.0;

    layer.use_text("INTEGRIDAD DE TRANSCRIPCIÓN", 12.0, Mm(20.0), Mm(y), &bold);
    y -= 6.0;
    layer.use_text(format!("Caracteres: {}", data.transcript_chars), 9.0, Mm(20.0), Mm(y), &font);
    y -= 5.0;
    layer.use_text("Hash SHA-256:", 9.0, Mm(20.0), Mm(y), &font);
    y -= 4.0;
    // Hash long: split en 2 líneas si necesario
    let hash = &data.transcript_hash;
    let mid = hash.len() / 2;
    layer.use_text(&hash[..mid], 8.0, Mm(20.0), Mm(y), &mono);
    y -= 4.0;
    layer.use_text(&hash[mid..], 8.0, Mm(20.0), Mm(y), &mono);
    y -= 12.0;

    let local_only = data.external_endpoints_detected.is_empty();
    let verdict = if local_only {
        "PROCESAMIENTO 100% LOCAL — CERO EGRESO DE DATOS"
    } else {
        "ATENCION: ENDPOINTS EXTERNOS DETECTADOS"
    };
    layer.use_text(verdict, 13.0, Mm(20.0), Mm(y), &bold);
    y -= 8.0;
    layer.use_text(
        format!("Endpoints locales utilizados: {}", data.local_endpoints_used.len()),
        10.0,
        Mm(20.0),
        Mm(y),
        &font,
    );
    y -= 5.0;
    for ep in &data.local_endpoints_used {
        layer.use_text(format!("  - {}", ep), 9.0, Mm(20.0), Mm(y), &mono);
        y -= 4.0;
    }
    y -= 4.0;

    if !data.external_endpoints_detected.is_empty() {
        layer.use_text("Endpoints externos:", 10.0, Mm(20.0), Mm(y), &bold);
        y -= 5.0;
        for ep in &data.external_endpoints_detected {
            layer.use_text(format!("  - {}", ep), 9.0, Mm(20.0), Mm(y), &mono);
            y -= 4.0;
        }
        y -= 4.0;
    }

    layer.use_text(
        format!("EVENTOS REGISTRADOS ({})", data.events.len()),
        12.0,
        Mm(20.0),
        Mm(y),
        &bold,
    );
    y -= 6.0;
    for ev in data.events.iter().take(40) {
        if y < 30.0 {
            // page break básico
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer1");
            let _ = (p, l);
            // Para simplicidad: parar aquí si pasa página 1.
            layer.use_text("(eventos truncados — ver SQLite audit_log)", 8.0, Mm(20.0), Mm(y), &font);
            break;
        }
        let line = format!(
            "[{}] {} -> {} ({})",
            ev.timestamp,
            ev.event_type,
            ev.endpoint.as_deref().unwrap_or("-"),
            ev.model.as_deref().unwrap_or("-")
        );
        layer.use_text(&line[..line.len().min(90)], 7.0, Mm(20.0), Mm(y), &mono);
        y -= 3.5;
    }

    let file = File::create(&final_path).map_err(|e| format!("crear PDF: {}", e))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| format!("save: {}", e))?;
    drop(writer);

    let bytes = std::fs::metadata(&final_path).map(|m| m.len() as usize).unwrap_or(0);
    Ok(ComplianceExportResult {
        path: final_path.to_string_lossy().to_string(),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_result_serializes() {
        let r = ComplianceExportResult { path: "/tmp/x.pdf".into(), bytes: 1234 };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("/tmp/x.pdf"));
    }

    #[test]
    fn compliance_event_serializes() {
        let ev = ComplianceEvent {
            event_type: "llm_call".to_string(),
            endpoint: Some("http://localhost:11434".to_string()),
            model: Some("phi".to_string()),
            timestamp: 1704067200000,
            metadata: Some(r#"{"tokens":100}"#.to_string()),
        };
        let s = serde_json::to_string(&ev).unwrap();
        assert!(s.contains("llm_call"));
        assert!(s.contains("localhost"));
    }

    #[test]
    fn compliance_audit_serializes() {
        let audit = ComplianceMeetingAudit {
            meeting_id: "m123".to_string(),
            transcript_hash: "abcd1234".to_string(),
            transcript_chars: 1000,
            event_count: 5,
            external_endpoints_detected: vec![],
            local_endpoints_used: vec!["http://localhost:11434".to_string()],
            events: vec![],
        };
        let s = serde_json::to_string(&audit).unwrap();
        assert!(s.contains("m123"));
        assert!(s.contains("localhost"));
        assert_eq!(audit.external_endpoints_detected.len(), 0);
    }
}
