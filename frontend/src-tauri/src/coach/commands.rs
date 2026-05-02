//! Comandos Tauri del copiloto IA.
//!
//! Contiene todos los comandos Tauri. Reusa tipos y helpers de:
//! - types: CoachSuggestion, CoachStatus, CoachModelsConfig
//! - parser: parse_llm_output, infer_tip_type
//! - model_state: gestión de modelos Ollama

use crate::coach::context::{build_context, ContextMode};

// Re-exportar para mantener compatibilidad con módulos que importan desde aquí
pub use crate::coach::model_state::{
    CHAT_MODEL, CURRENT_MODEL, EVALUATION_MODEL, SHARED_CLIENT,
};
pub use crate::coach::types::{CoachModelsConfig, CoachStatus, CoachSuggestion};

use crate::coach::model_state::{LAST_LATENCY_MS, check_ollama_running};
use crate::coach::parser::{infer_tip_type, parse_llm_output};
use crate::coach::prompt::{
    build_user_prompt_v3, MAITY_COPILOTO_V3_LITE_PROMPT, MeetingType,
};
use crate::coach::retry::{with_backoff, RetryConfig};
use crate::summary::llm_client::{generate_summary, LLMProvider};
use crate::validation_helpers;
use reqwest::Client;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::Manager;

/// Genera una sugerencia de coaching v3.0 con 31 frameworks + routing explícito.
///
/// # Argumentos
/// * `window` - Transcripción en vivo (frontend `buildWindow()`)
/// * `role` - Rol del usuario (compat, no usado en v3)
/// * `language` - Idioma (compat, el prompt v3 responde en el idioma del contexto)
/// * `meeting_id` - Opcional: ID de la reunión activa (lee de DB si existe)
/// * `meeting_type` - Opcional: "sales" | "service" | "webinar" | "team_meeting" | "auto"
/// * `minute` - Minuto actual de la sesión (para timing awareness del prompt)
/// * `previous_tips` - Lista de tips ya dados (para evitar repetición)
/// * `suggested_category` - Pista del trigger detector (opcional)
#[tauri::command]
pub async fn coach_suggest(
    app: tauri::AppHandle,
    window: String,
    role: String,
    language: String,
    meeting_id: Option<String>,
    meeting_type: Option<String>,
    minute: Option<u32>,
    previous_tips: Option<Vec<String>>,
    suggested_category: Option<String>,
    trigger_signal: Option<String>,
) -> Result<CoachSuggestion, String> {
    // Validate input parameters
    let _ = validation_helpers::validate_language(&role)?;
    let _ = validation_helpers::validate_language(&language)?;

    let validated_meeting_id = if let Some(mid) = meeting_id {
        Some(validation_helpers::validate_meeting_id(&mid)?)
    } else {
        None
    };

    let validated_meeting_type = if let Some(mt) = meeting_type {
        Some(validation_helpers::validate_string_length(&mt, "meeting_type", 50)?)
    } else {
        None
    };

    let validated_category = if let Some(cat) = suggested_category {
        Some(validation_helpers::validate_string_length(&cat, "suggested_category", 100)?)
    } else {
        None
    };

    let model = CURRENT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?
        .clone();

    // Window de contexto: tamaño máximo 600 chars para latencia <2s en CPU
    // v20: 600→1200 chars. qwen3:1.7b ctx 16384 maneja sin problema, duplica
    // contextualización de tips (ya validado: tips citan productos reales del audio).
    const WINDOW_CHAR_CAP: usize = 1200;
    let trimmed_window = if window.chars().count() > WINDOW_CHAR_CAP {
        let total: Vec<char> = window.chars().collect();
        let start = total.len().saturating_sub(WINDOW_CHAR_CAP);
        total[start..].iter().collect::<String>()
    } else {
        window.clone()
    };

    let effective_window = if !trimmed_window.trim().is_empty() {
        trimmed_window
    } else if let Some(mid) = validated_meeting_id.as_ref() {
        let state = app.try_state::<crate::state::AppState>();
        if let Some(app_state) = state {
            let pool = app_state.db_manager.pool();
            build_context(pool, mid, ContextMode::Full)
                .await
                .map(|c| c.formatted)
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mt = MeetingType::from_str_loose(validated_meeting_type.as_deref().unwrap_or("auto"));
    let prev = previous_tips.unwrap_or_default();
    let validated_signal = trigger_signal
        .map(|s| validation_helpers::validate_string_length(&s, "trigger_signal", 100))
        .transpose()?;

    let user_prompt = build_user_prompt_v3(
        &effective_window,
        mt,
        minute.unwrap_or(0),
        &prev,
        validated_category.as_deref(),
        validated_signal.as_deref(),
    );

    log::info!(
        "[coach_suggest v3] meeting_type={:?}, minute={}, window_chars={}, prev_tips={}, hint={:?}",
        mt,
        minute.unwrap_or(0),
        effective_window.len(),
        prev.len(),
        validated_category
    );

    let start = Instant::now();

    crate::progress_events::emit_coach_thinking(
        &app,
        crate::progress_events::CoachStage::Analyzing,
        0,
        &model,
    );

    let retry_config = RetryConfig {
        max_attempts: 2,
        initial_backoff_ms: 800,
        max_total_ms: 12_000,
        backoff_multiplier: 2.0,
    };

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No se pudo obtener app_data_dir: {}", e))?;

    let raw_result = with_backoff(&retry_config, "coach_suggest", |_attempt| {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(2)
            .build();

        let user_prompt_clone = user_prompt.clone();
        let model_clone = model.clone();
        let data_dir_clone = app_data_dir.clone();

        async move {
            let client = client.map_err(|e| format!("Failed to create HTTP client: {}", e))?;
            generate_summary(
                &client,
                &LLMProvider::BuiltInAI,
                &model_clone,
                "",
                MAITY_COPILOTO_V3_LITE_PROMPT,
                &user_prompt_clone,
                None,
                None,
                Some(50),
                Some(0.3),
                Some(0.7),
                Some(&data_dir_clone),
                None,
            )
            .await
        }
    })
    .await;

    let raw = match raw_result {
        Ok(r) => r,
        Err(e) => {
            crate::progress_events::emit_coach_thinking(
                &app,
                crate::progress_events::CoachStage::Error,
                start.elapsed().as_millis() as u64,
                &model,
            );
            return Err(format!(
                "Coach IA no responde — Ollama puede estar saturado o el modelo no disponible: {}",
                e
            ));
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    LAST_LATENCY_MS.store(latency_ms, Ordering::Relaxed);

    crate::progress_events::emit_coach_thinking(
        &app,
        crate::progress_events::CoachStage::Done,
        latency_ms,
        &model,
    );

    log::info!("[coach_suggest] raw LLM output ({} chars): {}", raw.len(), raw.chars().take(400).collect::<String>());
    let parsed = match parse_llm_output(&raw) {
        Ok(p) => {
            log::info!("[coach_suggest] parsed OK — tip='{}', confidence={}", p.tip, p.confidence);
            p
        }
        Err(e) => {
            log::warn!("[coach_suggest] parse FAILED: {}", e);
            return Err(e);
        }
    };

    // Derivar priority si el LLM no la provee, basado en confidence.
    let priority = parsed.priority.unwrap_or_else(|| {
        if parsed.confidence >= 0.85 {
            "critical".to_string()
        } else if parsed.confidence >= 0.6 {
            "important".to_string()
        } else {
            "soft".to_string()
        }
    });

    // V3.1: tip_type del LLM o inferido.
    let tip_type = parsed
        .tip_type
        .filter(|t| matches!(t.as_str(), "recognition" | "observation" | "corrective" | "introspective"))
        .unwrap_or_else(|| infer_tip_type(&parsed.tip, &priority));

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let suggestion = CoachSuggestion {
        tip: parsed.tip.clone(),
        category: parsed.category.clone(),
        subcategory: parsed.subcategory.clone(),
        technique: parsed.technique.clone(),
        priority: priority.clone(),
        confidence: parsed.confidence,
        tip_type: tip_type.clone(),
        timestamp,
        model: model.clone(),
        latency_ms,
    };

    // v26: persistir tip a coach_tips_log para histórico permanente.
    // Esto permite al dashboard mostrar tips reales (no solo tip_tests).
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let pool = state.db_manager.pool();
        let _ = sqlx::query(
            "INSERT INTO coach_tips_log (
                meeting_id, tip, category, subcategory, technique, priority,
                tip_type, confidence, latency_ms, model, minute, trigger_signal,
                suggested_category
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&validated_meeting_id)
        .bind(&parsed.tip)
        .bind(&parsed.category)
        .bind(&parsed.subcategory)
        .bind(&parsed.technique)
        .bind(&priority)
        .bind(&tip_type)
        .bind(parsed.confidence as f64)
        .bind(latency_ms as i64)
        .bind(&model)
        .bind(minute.map(|m| m as i64).unwrap_or(0))
        .bind(&validated_signal)
        .bind(&validated_category)
        .execute(pool)
        .await;
    }

    Ok(suggestion)
}

/// Cambia el modelo activo del coach.
///
/// Acepta cualquier modelo Ollama instalado (validación delegada al runtime).
/// Esto permite al usuario usar gemma4, gemma3, qwen3, codegemma, etc. sin
/// que el código tenga que enumerar la lista.
#[tauri::command]
pub fn coach_set_model(model_id: String) -> Result<(), String> {
    if model_id.trim().is_empty() {
        return Err("Modelo vacío".to_string());
    }
    let mut current = CURRENT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    *current = model_id;
    Ok(())
}

/// Devuelve los 3 modelos configurados (tips + evaluación + chat).
#[tauri::command]
pub fn coach_get_models() -> Result<CoachModelsConfig, String> {
    let tips_model = CURRENT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado tips: {}", e))?
        .clone();
    let evaluation_model = EVALUATION_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado eval: {}", e))?
        .clone();
    let chat_model = CHAT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado chat: {}", e))?
        .clone();
    Ok(CoachModelsConfig {
        tips_model,
        evaluation_model,
        chat_model,
    })
}

/// Cambia el modelo de un propósito específico.
/// `purpose` debe ser "tips" | "evaluation" | "chat".
#[tauri::command]
pub fn coach_set_model_for_purpose(purpose: String, model: String) -> Result<(), String> {
    if model.trim().is_empty() {
        return Err("Modelo vacío".to_string());
    }
    let target = match purpose.as_str() {
        "tips" => &CURRENT_MODEL,
        "evaluation" => &EVALUATION_MODEL,
        "chat" => &CHAT_MODEL,
        other => return Err(format!("Propósito inválido: {} (válidos: tips/evaluation/chat)", other)),
    };
    let mut current = target
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    *current = model;
    Ok(())
}

/// Devuelve el estado del coach: modelo activo, Ollama corriendo, última latencia.
#[tauri::command]
pub async fn coach_get_status() -> Result<CoachStatus, String> {
    let model = CURRENT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?
        .clone();

    let ollama_running = check_ollama_running().await;
    let last_latency_ms = LAST_LATENCY_MS.load(std::sync::atomic::Ordering::Relaxed);

    Ok(CoachStatus {
        model,
        ollama_running,
        last_latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_model_acepta_cualquier_ollama() {
        // Tras el fix de la asamblea (2026-04-11), aceptamos cualquier modelo
        // Ollama instalado. Solo rechazamos string vacío.
        assert!(coach_set_model("gemma4:latest".to_string()).is_ok());
        assert!(coach_set_model("gemma3:4b".to_string()).is_ok());
        assert!(coach_set_model("qwen3:8b".to_string()).is_ok());
        assert!(coach_set_model("custom-model:v2".to_string()).is_ok());
        assert!(coach_set_model("".to_string()).is_err());
        assert!(coach_set_model("   ".to_string()).is_err());
    }

    #[test]
    fn test_set_model_for_purpose_valida_proposito() {
        assert!(coach_set_model_for_purpose("tips".to_string(), "phi3.5:latest".to_string()).is_ok());
        assert!(coach_set_model_for_purpose("evaluation".to_string(), "gemma3:4b".to_string()).is_ok());
        assert!(coach_set_model_for_purpose("chat".to_string(), "qwen3:8b".to_string()).is_ok());
        assert!(coach_set_model_for_purpose("invalido".to_string(), "phi3.5".to_string()).is_err());
        assert!(coach_set_model_for_purpose("tips".to_string(), "".to_string()).is_err());
    }

    #[test]
    fn test_get_models_devuelve_los_tres() {
        coach_set_model_for_purpose("tips".to_string(), "tips-model".to_string()).unwrap();
        coach_set_model_for_purpose("evaluation".to_string(), "eval-model".to_string()).unwrap();
        coach_set_model_for_purpose("chat".to_string(), "chat-model".to_string()).unwrap();
        let cfg = coach_get_models().unwrap();
        assert_eq!(cfg.tips_model, "tips-model");
        assert_eq!(cfg.evaluation_model, "eval-model");
        assert_eq!(cfg.chat_model, "chat-model");
    }
}
