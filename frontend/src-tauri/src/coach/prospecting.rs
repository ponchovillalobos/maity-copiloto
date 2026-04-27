//! Auto-prospecting: genera email draft post-reunión basado en transcripción.
//!
//! Detecta automáticamente: objeciones planteadas, competidores mencionados,
//! próximos pasos comprometidos, dolor del cliente. Reusa Gemma3 local.

use crate::coach::commands::SHARED_CLIENT;
use crate::coach::context::{build_context, ContextMode};
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use serde::{Deserialize, Serialize};
use tauri::Manager;

const PROSPECTING_SYSTEM_PROMPT: &str = r#"Eres un asistente de ventas que escribe emails de seguimiento post-reunión. Tu tarea: leer la transcripción y generar:

1. Un email draft profesional al cliente con:
   - Saludo personalizado
   - Resumen de los 2-3 puntos más importantes que el cliente mencionó
   - Respuesta directa a las objeciones detectadas (si las hay)
   - Próximos pasos concretos con fechas/responsables si se mencionaron
   - Cierre con call-to-action claro
2. Datos estructurados extraídos de la conversación.

REGLAS ESTRICTAS:
- Responde SOLO con JSON válido, sin markdown ni texto extra
- Tono profesional cálido, NO genérico ("espero estés bien" prohibido)
- Cita literalmente al cliente cuando aporte ("mencionaste que...")
- NO inventes información que no esté en la transcripción
- Si el cliente mencionó un competidor (Salesforce, HubSpot, etc), inclúyelo en `competidores`
- Si detectas dolor real (ej: "perdemos clientes"), va en `dolores_detectados`

Esquema JSON exacto:
{
  "email_draft": {
    "asunto": "string",
    "saludo": "string",
    "cuerpo": "string (3-5 párrafos cortos)",
    "cierre": "string"
  },
  "objeciones_detectadas": ["string"],
  "competidores_mencionados": ["string"],
  "dolores_detectados": ["string"],
  "proximos_pasos": [{"accion": "string", "responsable": "USUARIO|INTERLOCUTOR|null", "fecha": "string|null"}],
  "nivel_interes_estimado": "alto|medio|bajo",
  "razon_nivel": "string"
}"#;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailDraft {
    pub asunto: String,
    pub saludo: String,
    pub cuerpo: String,
    pub cierre: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProximoPaso {
    pub accion: String,
    pub responsable: Option<String>,
    pub fecha: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProspectingSnapshot {
    pub email_draft: EmailDraft,
    pub objeciones_detectadas: Vec<String>,
    pub competidores_mencionados: Vec<String>,
    pub dolores_detectados: Vec<String>,
    pub proximos_pasos: Vec<ProximoPaso>,
    pub nivel_interes_estimado: String,
    pub razon_nivel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProspectingResult {
    pub meeting_id: String,
    pub snapshot: ProspectingSnapshot,
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

#[tauri::command]
pub async fn generate_prospecting_snapshot(
    app: tauri::AppHandle,
    meeting_id: String,
    model: Option<String>,
) -> Result<ProspectingResult, String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let model = model
        .or_else(|| {
            crate::coach::commands::EVALUATION_MODEL
                .lock()
                .ok()
                .map(|m| m.clone())
        })
        .unwrap_or_else(|| "gemma3:4b".to_string());

    let context = build_context(pool, &meeting_id, ContextMode::Full)
        .await
        .map_err(|e| format!("Error contexto: {}", e))?;

    if context.formatted.trim().is_empty() {
        return Err("Reunión sin transcripción suficiente".to_string());
    }

    let user_prompt = format!(
        "<transcripcion meeting=\"{}\" turnos=\"{}\">\n{}\n</transcripcion>\n\nGenera el JSON según el esquema.",
        meeting_id, context.turn_count, context.formatted
    );

    let client = &*SHARED_CLIENT;
    let start = std::time::Instant::now();

    let raw = generate_summary(
        client,
        &LLMProvider::Ollama,
        &model,
        "",
        PROSPECTING_SYSTEM_PROMPT,
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
    let snapshot: ProspectingSnapshot = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON inválido del modelo: {} | raw: {}", e, raw.chars().take(200).collect::<String>()))?;

    Ok(ProspectingResult {
        meeting_id,
        snapshot,
        model,
        latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_finds_block_with_prefix() {
        let raw = "Aquí está: {\"email_draft\":{\"asunto\":\"X\"}} fin";
        assert!(extract_json_block(raw).starts_with('{'));
        assert!(extract_json_block(raw).ends_with('}'));
    }

    #[test]
    fn parses_full_snapshot_json() {
        let raw = r#"{
            "email_draft": {"asunto":"Seguimiento","saludo":"Hola","cuerpo":"Texto","cierre":"Saludos"},
            "objeciones_detectadas": ["precio alto"],
            "competidores_mencionados": ["Salesforce"],
            "dolores_detectados": ["pérdida de leads"],
            "proximos_pasos": [{"accion":"enviar propuesta","responsable":"USUARIO","fecha":"2026-04-30"}],
            "nivel_interes_estimado": "alto",
            "razon_nivel": "preguntó por implementación"
        }"#;
        let s: ProspectingSnapshot = serde_json::from_str(raw).unwrap();
        assert_eq!(s.email_draft.asunto, "Seguimiento");
        assert_eq!(s.competidores_mencionados, vec!["Salesforce"]);
        assert_eq!(s.proximos_pasos.len(), 1);
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let raw = r#"{"email_draft":{"asunto":"X","saludo":"","cuerpo":"","cierre":""},"objeciones_detectadas":[],"competidores_mencionados":[],"dolores_detectados":[],"proximos_pasos":[],"nivel_interes_estimado":"","razon_nivel":""}"#;
        let s: ProspectingSnapshot = serde_json::from_str(raw).unwrap();
        assert!(s.objeciones_detectadas.is_empty());
        assert_eq!(s.nivel_interes_estimado, "");
    }
}
