//! Comandos Tauri del copiloto IA — única ruta v31.6.
//!
//! coach_simple_tick (cada 30s) → sidecar local → INSERT DB → polling burbuja.

pub use crate::coach::model_state::{
    CHAT_MODEL, CURRENT_MODEL, EVALUATION_MODEL, SHARED_CLIENT,
};
pub use crate::coach::types::{CoachModelsConfig, CoachStatus, CoachSuggestion};

use crate::coach::model_state::{check_ollama_running, LAST_LATENCY_MS};
use crate::summary::llm_client::{generate_summary, LLMProvider};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Manager;

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
    // v31.5: lock contra concurrencia con AtomicBool (sin lifetime issues).
    // compare_exchange atómico: si false→true OK, sino otra invocación está
    // en vuelo y skipeamos. Helper RAII reset que captura el AppHandle por
    // clone para liberar al final del scope (incluyendo returns tempranos).
    struct AtomicFlightGuard {
        app: tauri::AppHandle,
    }
    impl Drop for AtomicFlightGuard {
        fn drop(&mut self) {
            if let Some(state) = self.app.try_state::<crate::state::AppState>() {
                state
                    .coach_tick_in_flight
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }
    let _flight_guard = if let Some(state) = app.try_state::<crate::state::AppState>() {
        match state.coach_tick_in_flight.compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        ) {
            Ok(_) => Some(AtomicFlightGuard { app: app.clone() }),
            Err(_) => {
                log::info!("[coach_simple_tick] tick concurrente en vuelo — skip (auto/manual rate-limit)");
                return Ok(None);
            }
        }
    } else {
        None
    };

    // v31.2: si window viene vacío (típicamente desde la burbuja flotante
    // que no tiene acceso a transcriptsRef), construir desde el buffer
    // live_transcript del AppState que TranscriptContext alimenta.
    let mut effective_window = window.trim().to_string();
    if effective_window.len() < 30 {
        if let Some(state) = app.try_state::<crate::state::AppState>() {
            if let Ok(buf) = state.live_transcript.lock() {
                if !buf.is_empty() {
                    let mut lines: Vec<String> = Vec::with_capacity(buf.len());
                    for (_sid, speaker, text) in buf.iter() {
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

    // v31.8: meeting_id obligatorio (param explícito o AppState). Sin él,
    // los tips quedarían huérfanos en DB invisibles para la burbuja
    // (que pollea por activeMeetingId). Mejor return Ok(None) con log.
    let resolved_meeting_id: String = if let Some(mid) = meeting_id.filter(|s| !s.is_empty()) {
        mid
    } else if let Some(state) = app.try_state::<crate::state::AppState>() {
        match state.active_meeting_id.lock().ok().and_then(|g| g.clone()) {
            Some(mid) => mid,
            None => {
                log::warn!("[coach_simple_tick] sin active_meeting_id — skip (burbuja no podría ver el tip)");
                return Ok(None);
            }
        }
    } else {
        log::error!("[coach_simple_tick] AppState NO disponible — skip");
        return Ok(None);
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

    // v31.22: prompt simplificado del auditor + parser tolerante en código.
    // Acepta `Verbo: "frase"`, `Verbo: frase`, `Verbo - frase` y normaliza.
    let system_prompt = "/no_think\nEres Maity, coach en vivo del vendedor. Entrega una frase exacta para decir al cliente ahora.";
    let user_prompt = format!(
        "Transcript (USUARIO = vendedor; INTERLOCUTOR = cliente):\n\n{}\n\n\
         Formato preferido:\n\
         VERBO: \"frase breve\"\n\n\
         Verbos: Pregunta, Refleja, Valida, Reconoce, Aclara, Conecta, Profundiza, Escucha.\n\n\
         Reglas:\n\
         - La frase debe tener 5 a 15 palabras.\n\
         - Usa datos del transcript sólo si aparecen ahí.\n\
         - No inventes nombres, cifras, promesas ni emociones extremas.\n\
         - Si hay objeción, valida primero y haz pregunta abierta.\n\
         - Si hay interés claro, sugiere un siguiente paso suave.\n\
         - Si no hay base útil, responde: SIN_TIP\n\n\
         Tip:",
        window_capped
    );

    // v31.9: timeout 20→45s. CRITICAL FIX: con cold-start del sidecar
    // (qwen3:1.7b carga ~10s + genera ~12-15s en CPU = ~25-30s primera vez).
    // Timeout 20s mataba sidecar ANTES de responder → ciclo infinito spawn/kill
    // sin un solo tip generado. 45s da margen para cold-start; ticks siguientes
    // (sidecar warm) responden en <15s sin acercarse al límite.
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_for_timeout = cancel.clone();
    let timeout_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(45)).await;
        cancel_for_timeout.cancel();
    });

    // v31.22: BuiltInAI ignora el HTTP client. Usamos el SHARED_CLIENT global
    // para no crear una instancia nueva por tick (sobra memoria + sockets).
    let raw_result = generate_summary(
        &SHARED_CLIENT,
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

    // v31.22: strip <think>...</think> de Qwen3 ANTES de parsear.
    // Sin esto, el primer token podría ser "<think>" → tip rechazado por filtro.
    let raw_clean = strip_qwen3_thinking(&raw);
    let mut tip_text = raw_clean.trim().to_string();
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
    tip_text = tip_text.trim().to_string();
    // Quitar prefijos comunes
    for prefix in &["Tip:", "TIP:", "tip:", "Consejo:", "Sugerencia:"] {
        if let Some(rest) = tip_text.strip_prefix(prefix) {
            tip_text = rest.trim().to_string();
            break;
        }
    }
    // v31.22: normalizar formato. Acepta "VERBO: frase", "VERBO - frase",
    // "VERBO: \"frase\"" y deja siempre como `Verbo: "frase"`.
    tip_text = normalize_tip_format(&tip_text);
    if tip_text.is_empty() {
        return Ok(None);
    }
    // v30.1: filtro mínimo de calidad. Rechaza outputs basura del modelo.
    let word_count = tip_text.split_whitespace().count();
    if word_count < 5 {
        log::info!("[coach_simple_tick] tip rechazado: {} palabras < 5 (\"{}\")", word_count, tip_text);
        return Ok(None);
    }
    // v31.10: cap 22 palabras (verbo + ":" + "5-15 palabras frase" + puntuación).
    // No truncar agresivo: rechazar si excede mucho, mejor pedir nuevo tip.
    if word_count > 22 {
        log::info!("[coach_simple_tick] tip rechazado: {} palabras > 22 (formato pide 5-15 dentro de comillas)", word_count);
        return Ok(None);
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
    // v31.6: solo verbos empáticos. Quitados "cierra", "propón", "vende" que
    // sugieren presión. El coach SIEMPRE invita a escuchar/conectar/entender.
    let verbos_validos = [
        // Conectar
        "refleja", "espeja", "mirror", "etiqueta", "valida", "reconoce",
        "acompaña", "acompana", "acepta", "abraza", "asiente", "concede",
        "empatiza", "comprende", "humaniza", "personaliza", "conecta",
        // Entender
        "pregunta", "preguntá", "indaga", "explora", "aclara", "profundiza",
        "cuestiona", "verifica", "confirma", "escucha", "anota",
        // Calmar / desactivar
        "respira", "calma", "tranquiliza", "espera", "silencio",
        // Reformular sin presionar
        "reformula", "reformúlale", "reformulale", "devuelve", "resume",
        "cita", "menciona", "señala",
    ];
    if !verbos_validos.contains(&primera_palabra.as_str()) {
        log::info!("[coach_simple_tick] tip rechazado: no empieza con verbo imperativo (\"{}\")", primera_palabra);
        return Ok(None);
    }
    // v31.10: exigir formato accionable VERBO + ":" + frase entre comillas.
    // Sin esto, el modelo describe situación ("Refleja: El cliente expresa
    // preocupación...") en lugar de dar texto decible al cliente.
    let has_colon = tip_text.contains(':');
    let has_quotes = tip_text.contains('"') || tip_text.contains('\u{201C}') || tip_text.contains('\u{201D}');
    if !has_colon || !has_quotes {
        log::info!("[coach_simple_tick] tip rechazado: sin formato accionable VERBO:\"frase\" (\"{}\")", tip_text);
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

    let state = app
        .try_state::<crate::state::AppState>()
        .ok_or_else(|| "AppState no disponible para INSERT".to_string())?;
    let pool = state.db_manager.pool();

    // v31.7: dedup post-LLM. Si el modelo repite "Reconoce: Entiendo que esto
    // puede ser frustrante..." 5 veces seguidas, no lo guardamos. Comparamos
    // contra los últimos 3 tips del meeting actual con Jaccard sobre tokens.
    // Threshold 0.55 = >55% palabras compartidas → repetitivo.
    use sqlx::Row;
    if let Ok(rows) = sqlx::query("SELECT tip FROM coach_tips_log WHERE meeting_id = ? ORDER BY id DESC LIMIT 3")
        .bind(&resolved_meeting_id)
        .fetch_all(pool)
        .await
    {
        let new_lower = tip_text.to_lowercase();
        let new_tokens: std::collections::HashSet<&str> = new_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .collect();
        for row in &rows {
            if let Ok(prev) = row.try_get::<String, _>("tip") {
                let prev_lower = prev.to_lowercase();
                let prev_tokens: std::collections::HashSet<&str> = prev_lower
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|s| s.len() > 2)
                    .collect();
                let inter = new_tokens.intersection(&prev_tokens).count();
                let union = new_tokens.union(&prev_tokens).count().max(1);
                let jaccard = inter as f32 / union as f32;
                if jaccard > 0.55 {
                    log::info!(
                        "[coach_simple_tick] tip rechazado: jaccard {:.2} > 0.55 vs tip previo (\"{}\")",
                        jaccard,
                        prev.chars().take(50).collect::<String>()
                    );
                    return Ok(None);
                }
            }
        }
    }

    sqlx::query(
        "INSERT INTO coach_tips_log (meeting_id, tip, category, priority, tip_type, confidence, model, trigger_signal)
         VALUES (?, ?, 'general', 'soft', 'observation', 0.7, ?, 'simple_tick')",
    )
    .bind(&resolved_meeting_id)
    .bind(&tip_text)
    .bind(&model)
    .execute(pool)
    .await
    .map_err(|e| {
        log::error!("[coach_simple_tick] INSERT falló: {}", e);
        format!("DB INSERT failed: {}", e)
    })?;

    log::info!("[coach_simple_tick] tip OK ({} chars)", tip_text.len());
    Ok(Some(suggestion))
}

/// v31.2: alimenta el buffer live_transcript del AppState. Llamado por
/// TranscriptContext cada vez que llega un transcript-update. Permite que
/// la burbuja flotante (que no tiene acceso al transcriptsRef del frontend
/// principal) pida tips manuales sin construir su propio window.
///
/// v31.3.1 (2026-05-02): dedup interno. Como ahora aceptamos parciales
/// (TranscriptContext.tsx:372 sin filtro is_partial), parciales sucesivos
/// del mismo chunk pueden duplicarse en el buffer ("hola", "hola como",
/// "hola como estás"). Sin dedup, el LLM ve contexto repetido y emite el
/// mismo tip dos veces seguidas. Solución: si el nuevo text es prefijo o
/// suffix del último, REEMPLAZAR el último; si es exacto, ignorar.
#[tauri::command]
pub async fn coach_push_transcript_chunk(
    app: tauri::AppHandle,
    sequence_id: u64,
    speaker: String,
    text: String,
    is_partial: bool,
) -> Result<(), String> {
    let _ = is_partial; // sólo informativo en el caller; backend usa sequence_id
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

    // v31.8: dedup por sequence_id. Mismo evento re-emitido (parcial→final,
    // VAD jitter) reemplaza la entrada existente. Distinto sequence_id =
    // distinto chunk, agregar. Sin prefix matching cruzado entre chunks.
    if let Some(idx) = buf.iter().position(|(sid, _, _)| *sid == sequence_id) {
        buf[idx] = (sequence_id, speaker, trimmed.to_string());
        return Ok(());
    }

    buf.push_back((sequence_id, speaker, trimmed.to_string()));
    while buf.len() > 40 {
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

/// v31.22: strip <think>...</think> tags de Qwen3. Si Qwen emite razonamiento
/// antes del tip, sin esto el primer token rompe el filtro.
fn strip_qwen3_thinking(raw: &str) -> String {
    let mut out = raw.to_string();
    while let (Some(start), Some(end)) = (out.find("<think>"), out.find("</think>")) {
        if start < end {
            let before = &out[..start];
            let after = &out[end + "</think>".len()..];
            out = format!("{}{}", before, after);
        } else {
            break;
        }
    }
    // Si quedó <think> sin cierre, recortar desde ahí (modelo truncado mid-thinking).
    if let Some(idx) = out.find("<think>") {
        out = out[..idx].to_string();
    }
    out.trim().to_string()
}

/// v31.22: normaliza formato del tip. Acepta:
///   - `Verbo: "frase"`         → ya OK
///   - `Verbo: frase`            → agrega comillas
///   - `Verbo - frase`           → cambia a `:` y agrega comillas
///   - `Verbo. frase`            → cambia a `:` y agrega comillas
/// Si no encuentra patrón Verbo+separador, devuelve string original.
fn normalize_tip_format(input: &str) -> String {
    const VERBOS: &[&str] = &[
        "Pregunta", "Valida", "Aclara", "Refleja", "Reconoce",
        "Conecta", "Profundiza", "Escucha", "Indaga", "Explora",
    ];
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // Detectar verbo al inicio (case-insensitive en match, capitalización canonical en output).
    for verbo in VERBOS {
        let lower_trim = trimmed.to_lowercase();
        let lower_verbo = verbo.to_lowercase();
        if lower_trim.starts_with(&lower_verbo) {
            let rest = &trimmed[verbo.len()..];
            // Buscar separador: ":", "-", "."
            let after_sep = rest
                .trim_start()
                .strip_prefix(':')
                .or_else(|| rest.trim_start().strip_prefix('-'))
                .or_else(|| rest.trim_start().strip_prefix('.'))
                .unwrap_or_else(|| rest.trim_start())
                .trim();
            // Si la frase ya tiene comillas, dejar como está pero canonical
            let phrase = if after_sep.starts_with('"') && after_sep.ends_with('"') && after_sep.len() > 1 {
                after_sep.to_string()
            } else if after_sep.starts_with('\u{201C}') {
                // Comillas curvas → reemplazar por dobles
                after_sep.replace('\u{201C}', "\"").replace('\u{201D}', "\"")
            } else {
                // Sin comillas: envolver
                format!("\"{}\"", after_sep)
            };
            return format!("{}: {}", verbo, phrase);
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod normalize_tests {
    use super::*;

    #[test]
    fn test_normalize_with_quotes_ok() {
        let r = normalize_tip_format("Pregunta: \"¿cómo te sientes?\"");
        assert_eq!(r, "Pregunta: \"¿cómo te sientes?\"");
    }
    #[test]
    fn test_normalize_no_quotes() {
        let r = normalize_tip_format("Pregunta: ¿cómo te sientes?");
        assert_eq!(r, "Pregunta: \"¿cómo te sientes?\"");
    }
    #[test]
    fn test_normalize_dash_separator() {
        let r = normalize_tip_format("Valida - entiendo tu preocupación");
        assert_eq!(r, "Valida: \"entiendo tu preocupación\"");
    }
    #[test]
    fn test_strip_thinking() {
        let r = strip_qwen3_thinking("<think>razonamiento</think>\nPregunta: \"hola\"");
        assert_eq!(r, "Pregunta: \"hola\"");
    }
    #[test]
    fn test_strip_thinking_unclosed() {
        let r = strip_qwen3_thinking("<think>razonamiento sin cierre");
        assert_eq!(r, "");
    }
}

/// Devuelve el estado del coach: modelo activo, runtime IA listo, última latencia.
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
