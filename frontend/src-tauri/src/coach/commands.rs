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
use tokio_util::sync::CancellationToken;

/// BUG #9: timeout duro para tips del coach. Por encima de este umbral,
/// se cancela el sidecar para liberar `tipInFlightRef` en el frontend y
/// permitir que el siguiente heartbeat o request manual proceda.
const COACH_TIP_TIMEOUT_MS: u64 = 10_000;

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

    // BUG #9 fix: timeout duro de 10s con CancellationToken. Antes el sidecar
    // podía tardar hasta `GENERATION_TIMEOUT_SECS=900` (15 min) y dejaba el
    // `tipInFlightRef` del frontend bloqueado todo ese tiempo, congelando el
    // heartbeat y los disparos manuales. Ahora cualquier request que exceda
    // 10s se aborta vía CancellationToken (el sidecar lo respeta en client.rs:215).
    let cancel_token = CancellationToken::new();
    let cancel_token_for_timeout = cancel_token.clone();
    let timeout_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(COACH_TIP_TIMEOUT_MS)).await;
        cancel_token_for_timeout.cancel();
        log::warn!(
            "[coach_suggest] timeout {}ms — cancelando sidecar para liberar lock",
            COACH_TIP_TIMEOUT_MS
        );
    });

    let raw_result = with_backoff(&retry_config, "coach_suggest", |_attempt| {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(2)
            .build();

        let user_prompt_clone = user_prompt.clone();
        let model_clone = model.clone();
        let data_dir_clone = app_data_dir.clone();
        let cancel_token_clone = cancel_token.clone();

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
                Some(80), // BUG #1 fix: 80 tokens (≈12 palabras + JSON wrapper). Antes pasaba 35 pero el sidecar usaba 4096.
                Some(0.3),
                Some(0.7),
                Some(&data_dir_clone),
                Some(&cancel_token_clone),
            )
            .await
        }
    })
    .await;

    // Cancelar el timer si la generación terminó antes — evita un cancel() tardío
    // que afectaría requests futuros del shared sidecar.
    timeout_handle.abort();

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
                "Coach IA no responde — sidecar local llama-helper saturado o el modelo no se descargó: {}",
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
        id: None, // se llena en coach_get_recent_tips desde la columna id de la tabla
    };

    // v28 F5: aplicar filtros ANTES de INSERT. Antes Rust persistía TODO,
    // pero frontend filtraba después → dashboard veía tips que floating no.
    // Ahora: si Rust descarta, no INSERT, no devuelve tip. Sync total.
    let tip_lower = parsed.tip.to_lowercase();
    let vague_words = [
        "empatiza", "empatía", "rapport", "escucha activa", "sé empático",
        "muestra interés", "conecta con", "establece confianza",
    ];
    let is_vague = vague_words.iter().any(|w| tip_lower.contains(w))
        && !parsed.tip.contains('\'') && !parsed.tip.contains(':');
    let has_quoted_phrase = parsed.tip.contains('\'') || parsed.tip.contains(':');
    let needs_phrase = matches!(tip_type.as_str(), "corrective" | "observation");

    // BUG #2 fix: alineado con MIN_CONFIDENCE=0.30 del frontend (CoachContext.tsx:284).
    // Antes 0.55 silenciaba tips útiles que el frontend hubiera aceptado.
    if parsed.confidence < 0.30 {
        log::info!("[coach_suggest] descartado: confidence {} < 0.30", parsed.confidence);
        return Err(format!("Tip descartado por baja confianza ({})", parsed.confidence));
    }
    if is_vague {
        log::info!("[coach_suggest] descartado: tip vago/genérico: {}", parsed.tip);
        return Err("Tip descartado por vaguedad".to_string());
    }
    if needs_phrase && !has_quoted_phrase {
        log::info!("[coach_suggest] descartado: {} sin frase concreta: {}", tip_type, parsed.tip);
        return Err(format!("Tip {} requiere frase concreta entre comillas", tip_type));
    }

    // v26+v28: persistir tip a coach_tips_log SOLO si pasó filtros.
    // Garantiza que dashboard y floating vean los MISMOS tips.
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

    // BUG #12 fix (2026-05-02 runtime detection): emit `coach-tip-update`
    // DIRECTAMENTE desde el backend a TODAS las webviews + emit_to específico
    // a "coach-floating". El frontend también emite vía `pushSuggestion()`,
    // pero en Windows con webviews transparent+decorations:false el `emit()`
    // global del frontend a veces NO se propaga a la flotante (problema de
    // routing IPC observado en runtime: tips se generaban e insertaban en DB,
    // pero la burbuja flotante nunca los recibía).
    //
    // Doble estrategia para garantizar entrega:
    //   1. `app.emit(...)` — broadcast global a todas las webviews
    //   2. `app.emit_to("coach-floating", ...)` — directo al label de la
    //      burbuja, bypass del routing global
    //
    // BUG #12.1 (2026-05-02 análisis externo): el emit backend bypassa la
    // dedup del frontend → riesgo de mostrar tips repetidos. Mitigación:
    // dedup pre-emit verificando si el mismo tip exacto se emitió en los
    // últimos 60s del mismo meeting. Filtros más complejos (audience mode,
    // Jaccard >0.40) siguen en el frontend; el backend cubre solo el caso
    // común de "modelo genera el mismo string varias veces seguidas".
    use tauri::{Emitter, EventTarget};

    let mut should_emit = true;
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        if let Some(mid) = &validated_meeting_id {
            let pool = state.db_manager.pool();
            // Última fila INSERTada para este meeting (la actual). Comparamos
            // contra las anteriores recientes para detectar repetición exacta.
            let recent: Result<Vec<(String,)>, _> = sqlx::query_as(
                "SELECT tip FROM coach_tips_log
                 WHERE meeting_id = ? AND created_at > datetime('now','-60 seconds')
                 ORDER BY id DESC LIMIT 6 OFFSET 1",
            )
            .bind(mid)
            .fetch_all(pool)
            .await;
            if let Ok(rows) = recent {
                if rows.iter().any(|(t,)| t.trim() == suggestion.tip.trim()) {
                    log::info!("[coach_suggest] dedup backend: tip exacto repetido en últimos 60s, skip emit");
                    should_emit = false;
                }
            }
        }
    }

    if should_emit {
        let payload_json = match serde_json::to_value(&suggestion) {
            Ok(v) => v,
            Err(e) => {
                log::error!("[coach_suggest] serde JSON falló: {}", e);
                serde_json::json!({})
            }
        };
        if let Err(e) = app.emit("coach-tip-update", &payload_json) {
            log::warn!("[coach_suggest] fallo emit global coach-tip-update: {}", e);
        } else {
            log::info!("[coach_suggest] emit global coach-tip-update OK ({})", suggestion.tip.chars().take(50).collect::<String>());
        }
        if let Err(e) = app.emit_to(EventTarget::webview_window("coach-floating"), "coach-tip-update", &payload_json) {
            log::warn!("[coach_suggest] fallo emit_to coach-floating: {}", e);
        } else {
            log::info!("[coach_suggest] emit_to coach-floating OK");
        }
    }

    Ok(suggestion)
}

/// v28 F4: comando para que floating window cargue tips ya generados al abrir.
/// Sin este catch-up, si la ventana flotante abre tarde pierde el histórico.
#[tauri::command]
pub async fn coach_get_recent_tips(
    app: tauri::AppHandle,
    meeting_id: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<CoachSuggestion>, String> {
    use sqlx::Row;
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();
    let lim = limit.unwrap_or(50).min(200) as i64;

    let rows = if let Some(mid) = meeting_id {
        sqlx::query(
            "SELECT id, tip, category, subcategory, technique, priority, tip_type,
                    confidence, latency_ms, model, created_at
             FROM coach_tips_log
             WHERE meeting_id = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(mid)
        .bind(lim)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, tip, category, subcategory, technique, priority, tip_type,
                    confidence, latency_ms, model, created_at
             FROM coach_tips_log ORDER BY id DESC LIMIT ?",
        )
        .bind(lim)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| format!("DB error: {}", e))?;

    let tips: Vec<CoachSuggestion> = rows
        .iter()
        .map(|r| CoachSuggestion {
            tip: r.get("tip"),
            category: r.get("category"),
            subcategory: r.try_get("subcategory").ok(),
            technique: r.try_get("technique").ok(),
            priority: r.get::<String, _>("priority"),
            confidence: r.try_get::<f64, _>("confidence").unwrap_or(0.7) as f32,
            tip_type: r.get::<String, _>("tip_type"),
            // BUG #14 fix (2026-05-02 agente listener): antes hardcodeado a 0,
            // causaba que el dedup de la burbuja (page.tsx:150) tratara como
            // duplicado a tips live cuyo timestamp diferiera <5s al casual de
            // ambos ser 0. Parseamos `created_at` (formato ISO o "YYYY-MM-DD HH:MM:SS")
            // a Unix epoch en segundos.
            timestamp: r.try_get::<String, _>("created_at")
                .ok()
                .and_then(|s| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f"))
                    .ok()
                    .map(|dt| dt.and_utc().timestamp()))
                .unwrap_or(0),
            model: r.try_get("model").unwrap_or_default(),
            latency_ms: r.try_get::<i64, _>("latency_ms").unwrap_or(0) as u64,
            id: r.try_get::<i64, _>("id").ok(),
        })
        .collect();
    Ok(tips)
}

/// BUG #7 fix: durante la grabación los tips se guardan con `meeting-${Date.now()}`
/// (TranscriptContext.tsx:100), pero al cerrar la reunión SQLite recibe un UUID
/// distinto. Sin remap los tips quedan huérfanos — la vista de detalle muestra
/// "0 tips" aunque sí se generaron varios.
///
/// Este comando se invoca desde `useRecordingStop.ts` justo después de `saveMeeting`
/// con (temp_meeting_id, final_meeting_id) y reasigna los tips al ID definitivo.
#[tauri::command]
pub async fn coach_remap_meeting_id(
    app: tauri::AppHandle,
    temp_meeting_id: String,
    final_meeting_id: String,
) -> Result<u64, String> {
    if temp_meeting_id.trim().is_empty() || final_meeting_id.trim().is_empty() {
        return Err("temp_meeting_id y final_meeting_id no pueden estar vacíos".to_string());
    }
    if temp_meeting_id == final_meeting_id {
        return Ok(0);
    }
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();
    let result = sqlx::query("UPDATE coach_tips_log SET meeting_id = ? WHERE meeting_id = ?")
        .bind(&final_meeting_id)
        .bind(&temp_meeting_id)
        .execute(pool)
        .await
        .map_err(|e| format!("DB error remapeando tips: {}", e))?;
    let affected = result.rows_affected();
    log::info!(
        "[coach_remap_meeting_id] {} tips reasignados de {} a {}",
        affected,
        temp_meeting_id,
        final_meeting_id
    );
    Ok(affected)
}

/// BUG #13 fix (2026-05-02): puente cross-webview para "pedir tip" desde la
/// burbuja flotante. En Tauri 2 `emit()` desde frontend SOLO entrega listeners
/// en la misma webview. La burbuja "coach-floating" emitía `coach-request-tip`
/// pero CoachContext (en webview "main") nunca lo recibía → triggerNow NUNCA
/// se invocaba → 0 tips manuales por sesión. Este comando hace el puente:
/// burbuja invoca este Tauri command, Rust emite `coach-request-tip` con
/// `app.emit()` global que SÍ entrega a TODAS las webviews registradas
/// (verificado en Tauri 2.6.2 + capability `core:event:default`).
#[tauri::command]
pub async fn coach_request_tip_bridge(
    app: tauri::AppHandle,
    source: Option<String>,
) -> Result<(), String> {
    use tauri::Emitter;
    let payload = serde_json::json!({ "source": source.unwrap_or_else(|| "unknown".to_string()) });
    app.emit("coach-request-tip", &payload)
        .map_err(|e| format!("Fallo emit coach-request-tip: {}", e))?;
    log::info!("[coach_request_tip_bridge] emit coach-request-tip OK");
    Ok(())
}

/// SIMPLE LOOP v30 (2026-05-02): pipeline mínimo end-to-end.
///
/// Flujo:
///   1. Frontend (CoachContext) llama cada 30s con `window` ya construido a
///      partir de transcriptsRef (que se actualiza en vivo via
///      `transcript-update` event).
///   2. Backend invoca sidecar BuiltInAI con prompt simple.
///   3. INSERT a coach_tips_log.
///   4. `app.emit("coach-tip-update", suggestion)` → cualquier webview que
///      escuche (la burbuja en `/floating`) lo recibe.
///
/// NOTA: durante grabación los transcripts viven en memoria del frontend
/// (transcriptsRef en TranscriptContext) e IndexedDB. NO se persisten en
/// SQLite tabla `transcripts` hasta que el usuario detenga la grabación
/// (lo hace `save_transcript` en transcript.rs:13). Por eso este comando
/// recibe `window` desde el frontend en lugar de leer SQLite.
#[tauri::command]
pub async fn coach_simple_tick(
    app: tauri::AppHandle,
    window: String,
    meeting_id: Option<String>,
) -> Result<Option<CoachSuggestion>, String> {
    use tauri::Emitter;

    // v31.2: si window viene vacío (típicamente desde la burbuja flotante
    // que no tiene acceso a transcriptsRef), construir desde el buffer
    // live_transcript del AppState que TranscriptContext alimenta.
    let mut effective_window = window.trim().to_string();
    if effective_window.len() < 30 {
        if let Some(state) = app.try_state::<crate::state::AppState>() {
            if let Ok(buf) = state.live_transcript.lock() {
                if !buf.is_empty() {
                    let mut lines: Vec<String> = Vec::with_capacity(buf.len());
                    for (speaker, text) in buf.iter() {
                        let label = match speaker.as_str() {
                            "user" => "USUARIO",
                            "interlocutor" => "INTERLOCUTOR",
                            _ => "VOZ",
                        };
                        lines.push(format!("{}: {}", label, text));
                    }
                    effective_window = lines.join("\n");
                    log::info!("[coach_simple_tick] window vacío → fallback live_transcript ({} chunks, {} chars)", buf.len(), effective_window.len());
                }
            }
        }
    }
    let trimmed = effective_window.trim();
    if trimmed.len() < 30 {
        log::debug!("[coach_simple_tick] window+buffer vacíos, skip");
        return Ok(None);
    }

    // v31.1: cap a últimas 800 chars (era 1500). Menos contexto = menos
    // probabilidad de alucinación con qwen3:1.7b en CPU.
    let window_capped: String = if trimmed.chars().count() > 800 {
        let start = trimmed.chars().count().saturating_sub(800);
        trimmed.chars().skip(start).collect()
    } else {
        trimmed.to_string()
    };

    // Resolver meeting_id: param explícito > AppState > "live" fallback.
    // v31.3: warning visible si cae al fallback "live" — la burbuja consulta
    // por meeting_id activo y NO encontrará tips si quedan bajo "live".
    let resolved_meeting_id: String = if let Some(mid) = meeting_id.filter(|s| !s.is_empty()) {
        mid
    } else if let Some(state) = app.try_state::<crate::state::AppState>() {
        state
            .active_meeting_id
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| {
                log::warn!("[coach_simple_tick] active_meeting_id es None — fallback meeting_id='live' (la burbuja podría no encontrar este tip si su meeting actual es distinto)");
                "live".to_string()
            })
    } else {
        log::warn!("[coach_simple_tick] AppState NO disponible — fallback meeting_id='live'");
        "live".to_string()
    };

    // invoca sidecar — timeout duro 15s vía CancellationToken.
    let model = CURRENT_MODEL
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?
        .clone();
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No app_data_dir: {}", e))?;

    // v31.1: prompt anti-alucinación. Tips 93-95 mostraron que el modelo
    // inventa contenido absurdo ("caca", "pipí", "usuario inútil"). Forzamos
    // que el tip se base EXCLUSIVAMENTE en lo que está en el contexto.
    let system_prompt = "Eres Maity, coach de comunicación. SOLO puedes referirte a palabras y temas QUE APARECEN LITERALMENTE en el contexto que recibes. PROHIBIDO inventar, exagerar o usar palabras que no estén en el transcript. Si el contexto es ambiguo o no permite un consejo útil, responde solo: SIN_TIP";
    let user_prompt = format!(
        "TRANSCRIPT REAL DE LA CONVERSACIÓN (USUARIO = micrófono del vendedor que coacheas; INTERLOCUTOR = otro hablante):\n\n---\n{}\n---\n\n\
         Da UN consejo al USUARIO siguiendo ESTAS REGLAS ESTRICTAS:\n\
         1. Empieza con verbo imperativo: Pregunta, Reformula, Resume, Profundiza, Aclara, Confirma, Cierra, Propón, Valida, Reconoce, Escucha, Verifica, Comparte.\n\
         2. Entre 8 y 18 palabras.\n\
         3. SOLO menciona temas, palabras o nombres QUE APARECEN LITERALMENTE arriba en el TRANSCRIPT REAL. Si no aparecen, NO los uses.\n\
         4. Si el transcript no permite un tip útil (es muy corto, sin contenido claro, ruido), responde EXACTAMENTE: SIN_TIP\n\
         5. JAMÁS inventes citas, hechos, ni atribuyas insultos o juicios al INTERLOCUTOR si no están textualmente arriba.\n\
         6. Una sola línea. Sin JSON, sin prefijos, sin explicaciones.\n\n\
         Tu consejo (o SIN_TIP):",
        window_capped
    );

    // v31.3: timeout 15→20s. qwen3:1.7b en CPU (sin GPU) puede tardar 12-15s
    // por tip. El timeout previo cancelaba justo cuando el modelo estaba
    // retornando, perdiendo tips. 20s da margen mientras sigue siendo aceptable
    // para coaching cada 30s.
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_for_timeout = cancel.clone();
    let timeout_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        cancel_for_timeout.cancel();
    });

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let raw_result = generate_summary(
        &client,
        &LLMProvider::BuiltInAI,
        &model,
        "",
        system_prompt,
        &user_prompt,
        None,
        None,
        Some(80),
        Some(0.4), // temperatura baja → más determinismo, menos divagación
        Some(0.9),
        Some(&app_data_dir),
        Some(&cancel),
    )
    .await;
    timeout_handle.abort();

    let raw = match raw_result {
        Ok(t) => t,
        Err(e) => {
            log::warn!("[coach_simple_tick] LLM falló: {}", e);
            return Ok(None);
        }
    };

    // El modelo a veces ignora "sin JSON" y devuelve {"tip":"..."}.
    // Extraemos el contenido si detectamos ese patrón.
    let mut tip_text = raw.trim().to_string();
    // Quitar markdown fences si existen
    tip_text = tip_text.trim_start_matches("```json").trim_start_matches("```").to_string();
    tip_text = tip_text.trim_end_matches("```").trim().to_string();
    // Si parece JSON con un campo "tip", extraerlo
    if tip_text.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&tip_text) {
            if let Some(t) = v.get("tip").and_then(|x| x.as_str()) {
                tip_text = t.to_string();
            } else if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
                tip_text = t.to_string();
            } else if let Some(t) = v.get("response").and_then(|x| x.as_str()) {
                tip_text = t.to_string();
            }
        }
    }
    tip_text = tip_text.trim().trim_matches('"').to_string();
    // Quitar prefijos comunes
    for prefix in &["Tip:", "TIP:", "tip:", "Consejo:", "Sugerencia:"] {
        if let Some(rest) = tip_text.strip_prefix(prefix) {
            tip_text = rest.trim().to_string();
            break;
        }
    }
    if tip_text.is_empty() {
        return Ok(None);
    }
    // v30.1: filtro mínimo de calidad. Rechaza outputs basura del modelo.
    let word_count = tip_text.split_whitespace().count();
    if word_count < 5 {
        log::info!("[coach_simple_tick] tip rechazado: {} palabras < 5 (\"{}\")", word_count, tip_text);
        return Ok(None);
    }
    // v30.3: máximo 25 palabras (cap duro contra descripciones largas).
    if word_count > 25 {
        log::info!("[coach_simple_tick] tip truncado: {} palabras > 25", word_count);
        let truncado: String = tip_text.split_whitespace().take(22).collect::<Vec<_>>().join(" ");
        tip_text = format!("{}…", truncado);
    }
    // v30.3: filtro backend que exige verbo imperativo al inicio. Rechaza
    // descripciones tipo "Bien hecho:", "La inteligencia es…", "Es importante…"
    // que no son consejos accionables.
    let primera_palabra = tip_text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(|c: char| !c.is_alphabetic())
        .to_lowercase();
    let verbos_validos = [
        "pregunta", "preguntá", "reformula", "resume", "profundiza",
        "cita", "aclara", "confirma", "cierra", "propón", "propon",
        "valida", "reconoce", "espera", "explora", "cuestiona", "verifica",
        "agradece", "muestra", "comparte", "destaca", "menciona", "sugiere",
        "ofrece", "haz", "di", "responde", "escucha", "explica", "describe",
        "anota", "señala", "señala", "pide", "pídele", "pidele", "indaga",
    ];
    if !verbos_validos.contains(&primera_palabra.as_str()) {
        log::info!("[coach_simple_tick] tip rechazado: no empieza con verbo imperativo (\"{}\")", primera_palabra);
        return Ok(None);
    }
    // v31.1: rechaza el flag SIN_TIP que el modelo emite cuando no hay base
    // para coachear, y tips con contenido vulgar/ofensivo (anti-alucinación).
    let lower_full = tip_text.to_lowercase();
    if lower_full.contains("sin_tip") || lower_full == "sin tip" {
        log::info!("[coach_simple_tick] modelo respondió SIN_TIP — sin base para coachear");
        return Ok(None);
    }
    let palabras_prohibidas = [
        "caca", "pipí", "pipi", "mierda", "puta", "pendej", "carajo",
        "inútil", "estúpido", "idiota", "imbécil", "tonto", "imbecil",
    ];
    if palabras_prohibidas.iter().any(|p| lower_full.contains(p)) {
        log::warn!("[coach_simple_tick] tip rechazado: contenido vulgar/ofensivo (\"{}\")", tip_text);
        return Ok(None);
    }
    // Rechaza preguntas vacías genéricas
    let lower = tip_text.to_lowercase();
    let basura = [
        "¿qué?", "qué?", "¿qué es?", "qué es?", "¿cómo?", "cómo?",
        "explica el futuro", "explicación del futuro", "explica la importancia",
    ];
    if basura.iter().any(|b| lower.trim().trim_end_matches('?').trim() == b.trim_end_matches('?').trim()
        || lower == *b) {
        log::info!("[coach_simple_tick] tip rechazado: basura genérica (\"{}\")", tip_text);
        return Ok(None);
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let suggestion = CoachSuggestion {
        tip: tip_text.clone(),
        category: "general".to_string(),
        subcategory: None,
        technique: None,
        priority: "soft".to_string(),
        confidence: 0.7,
        tip_type: "observation".to_string(),
        timestamp,
        model: model.clone(),
        latency_ms: 0,
        id: None,
    };

    // INSERT a coach_tips_log — DB es la ÚNICA fuente de verdad para la
    // burbuja flotante (que pollea coach_get_recent_tips cada 3s).
    // v31: eliminado app.emit("coach-tip-update") — la burbuja descubre el
    // tip vía polling. Una sola ruta, sin race conditions.
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        let pool = state.db_manager.pool();
        let _ = sqlx::query(
            "INSERT INTO coach_tips_log (meeting_id, tip, category, priority, tip_type, confidence, model, trigger_signal)
             VALUES (?, ?, 'general', 'soft', 'observation', 0.7, ?, 'simple_tick')",
        )
        .bind(&resolved_meeting_id)
        .bind(&tip_text)
        .bind(&model)
        .execute(pool)
        .await;
    }

    log::info!("[coach_simple_tick] tip OK ({} chars)", tip_text.len());
    Ok(Some(suggestion))
}

/// v31.2: alimenta el buffer live_transcript del AppState. Llamado por
/// TranscriptContext cada vez que llega un transcript-update. Permite que
/// la burbuja flotante (que no tiene acceso al transcriptsRef del frontend
/// principal) pida tips manuales sin construir su propio window.
#[tauri::command]
pub async fn coach_push_transcript_chunk(
    app: tauri::AppHandle,
    speaker: String,
    text: String,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let mut buf = state
        .live_transcript
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    buf.push_back((speaker, trimmed.to_string()));
    while buf.len() > 60 {
        buf.pop_front();
    }
    Ok(())
}

/// v31.2: limpia el buffer live_transcript. Llamado al detener grabación.
#[tauri::command]
pub async fn coach_clear_live_transcript(app: tauri::AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let mut buf = state
        .live_transcript
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    buf.clear();
    Ok(())
}

/// v31: bridge cross-webview para "Pedir tip" desde la burbuja flotante.
/// Reemplaza al viejo `coach_request_tip_bridge` que disparaba el flujo
/// triggerNow del CoachContext. Ahora invoca directamente `coach_simple_tick`
/// con un window mínimo (la burbuja no tiene acceso a transcriptsRef).
/// Si el window es muy corto, el comando devolverá None — el usuario verá
/// el siguiente tip en el próximo tick automático.
#[tauri::command]
pub async fn coach_request_simple_tip(
    app: tauri::AppHandle,
    window: Option<String>,
    meeting_id: Option<String>,
) -> Result<Option<CoachSuggestion>, String> {
    let win = window.unwrap_or_default();
    coach_simple_tick(app, win, meeting_id).await
}

/// BUG #16 fix (asamblea 2026-05-02 — agente A6): Tauri 2 aísla sessionStorage
/// entre webviews de orígenes distintos. La webview "main" (`/`) y la burbuja
/// "coach-floating" (`/floating`) tienen sessionStorage AISLADOS. Por eso la
/// burbuja nunca podía leer `indexeddb_current_meeting_id` que el TranscriptContext
/// escribía desde la webview principal.
///
/// Solución: el `active_meeting_id` vive en AppState (compartido entre webviews
/// porque Rust gestiona ambas). Estos 3 comandos exponen set/get/clear desde
/// cualquier webview, eliminando dependencia del sessionStorage cross-webview.
#[tauri::command]
pub async fn set_active_meeting_id(
    app: tauri::AppHandle,
    meeting_id: String,
) -> Result<(), String> {
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let mut guard = state
        .active_meeting_id
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    *guard = Some(meeting_id.clone());
    log::info!("[set_active_meeting_id] meeting_id activo = {}", meeting_id);
    Ok(())
}

#[tauri::command]
pub async fn get_active_meeting_id(
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let guard = state
        .active_meeting_id
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    Ok(guard.clone())
}

#[tauri::command]
pub async fn clear_active_meeting_id(app: tauri::AppHandle) -> Result<(), String> {
    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let mut guard = state
        .active_meeting_id
        .lock()
        .map_err(|e| format!("Mutex envenenado: {}", e))?;
    *guard = None;
    log::info!("[clear_active_meeting_id] cleared");
    Ok(())
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

    // BUG #3 fix: el sistema usa sidecar BuiltInAI (`llama-helper.exe`), no Ollama.
    // `check_ollama_running` es código residual de la arquitectura previa. Reportamos
    // "IA disponible" si el sidecar local responde sano; si el usuario aún tiene
    // Ollama instalado lo aceptamos como segundo backend (OR).
    let sidecar_ready =
        crate::summary::summary_engine::is_sidecar_healthy().await || check_ollama_running().await;
    let last_latency_ms = LAST_LATENCY_MS.load(std::sync::atomic::Ordering::Relaxed);

    Ok(CoachStatus {
        model,
        ollama_running: sidecar_ready,
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
