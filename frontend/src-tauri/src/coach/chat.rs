//! Chat bidireccional con el coach IA.
//!
//! Permite al usuario hacer preguntas en lenguaje natural sobre la reunión
//! actual. La IA responde usando como contexto:
//!   1. La transcripción FULL de la reunión (no rolling window)
//!   2. Speaker segregation (USUARIO vs INTERLOCUTOR)
//!   3. Historial de la conversación con el coach (multi-turn)
//!
//! Reusa `summary::llm_client` con provider FIJO Ollama (privacidad).

use crate::coach::commands::SHARED_CLIENT;
use crate::coach::context::{build_context, ContextMode};
use crate::summary::llm_client::{generate_summary, LLMProvider};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

/// Contexto máximo para chat (rolling window). Antes era `Full` sin límite
/// lo que producía prompts de 10-30KB → prefill Ollama 4-6s.
/// 6000 chars ≈ 1500 tokens (suficiente para follow-up en reunión de 10min).
const CHAT_CONTEXT_MAX_CHARS: usize = 6_000;

const CHAT_SYSTEM_PROMPT: &str = r#"Eres un copiloto IA inteligente que acompaña al usuario durante una reunión en vivo. Tienes acceso a la transcripción completa de la conversación, segregada por speaker (USUARIO = el dueño del micrófono; INTERLOCUTOR = la otra persona).

REGLAS:
1. Responde DIRECTO, en español neutro, sin preámbulos ("¡claro!", "¡perfecto!").
2. Sé CONCRETO: si te piden un consejo, da el consejo. Si te piden análisis, analiza con datos del transcript.
3. CITA partes del transcript cuando sea relevante: "el cliente dijo 'X' en el min Y".
4. NO inventes datos que no estén en el transcript.
5. Máximo 4 oraciones por respuesta a menos que el usuario pida explícitamente más detalle.
6. Si el contexto del transcript es insuficiente para responder, dilo y sugiere qué escuchar.
7. Tono: profesional pero cercano, como un mentor que escuchó la reunión contigo."#;

#[derive(Debug, Deserialize)]
pub struct ChatHistoryEntry {
    pub role: String, // "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChatResponse {
    pub answer: String,
    pub model: String,
    pub latency_ms: u64,
    pub context_chars: usize,
    pub context_turns: usize,
    pub user_turns: usize,
    pub interlocutor_turns: usize,
}

/// Comando Tauri: chat con el coach.
///
/// # Argumentos
/// * `message` - Pregunta del usuario en lenguaje natural
/// * `meeting_id` - ID de la reunión actual (para leer transcript de DB)
/// * `history` - Historial de turnos previos del chat (opcional)
/// * `model` - Modelo Ollama a usar (default: gemma4:latest)
#[tauri::command]
pub async fn coach_chat(
    app: tauri::AppHandle,
    message: String,
    meeting_id: Option<String>,
    live_transcript: Option<String>,
    history: Option<Vec<ChatHistoryEntry>>,
    model: Option<String>,
) -> Result<ChatResponse, String> {
    log::info!(
        "🧠 coach_chat INVOKED: message={:?}, meeting_id={:?}, live_len={}, history_len={}, model={:?}",
        message.chars().take(50).collect::<String>(),
        meeting_id,
        live_transcript.as_ref().map(|s| s.len()).unwrap_or(0),
        history.as_ref().map(|h| h.len()).unwrap_or(0),
        model
    );

    if message.trim().is_empty() {
        return Err("Mensaje vacío".to_string());
    }

    let model_to_use = model.unwrap_or_else(|| crate::coach::prompt::DEFAULT_MODEL.to_string());

    // Prioridad de contexto:
    // 1. live_transcript del frontend (durante grabación en vivo, DB está vacía)
    // 2. DB vía meeting_id (reuniones ya guardadas)
    // 3. Contexto vacío (chat sin contexto, respuesta genérica)
    let context = if let Some(live) = live_transcript.filter(|s| !s.trim().is_empty()) {
        log::info!("[coach_chat] Usando live_transcript del frontend ({} chars)", live.len());
        let live_len = live.len();
        crate::coach::context::CoachContext {
            formatted: live,
            turn_count: 0,
            char_count: live_len,
            user_turns: 0,
            interlocutor_turns: 0,
        }
    } else if let Some(mid) = meeting_id.as_ref() {
        let state = app
            .try_state::<crate::state::AppState>()
            .ok_or_else(|| "AppState no disponible".to_string())?;
        let pool = state.db_manager.pool();
        build_context(pool, mid, ContextMode::Recent { max_chars: CHAT_CONTEXT_MAX_CHARS })
            .await
            .unwrap_or_else(|e| {
                log::warn!("[coach_chat] Error cargando contexto: {}", e);
                crate::coach::context::CoachContext::empty()
            })
    } else {
        crate::coach::context::CoachContext::empty()
    };

    // 2. Armar el user_prompt: contexto + historial + pregunta actual
    let mut user_prompt = String::new();

    if !context.is_empty() {
        user_prompt.push_str(&format!(
            "<transcripcion meeting=\"{}\" turnos=\"{}\" usuario_turnos=\"{}\" interlocutor_turnos=\"{}\">\n{}\n</transcripcion>\n\n",
            meeting_id.as_deref().unwrap_or("unknown"),
            context.turn_count,
            context.user_turns,
            context.interlocutor_turns,
            context.formatted
        ));
    } else {
        user_prompt.push_str("<transcripcion>(sin transcripción disponible)</transcripcion>\n\n");
    }

    // Historial de chat previo (multi-turn)
    if let Some(hist) = history {
        if !hist.is_empty() {
            user_prompt.push_str("<historial_chat>\n");
            for entry in hist.iter().take(20) {
                // cap a últimos 20 turnos
                let role_label = if entry.role == "user" { "Usuario" } else { "Coach" };
                user_prompt.push_str(&format!("{}: {}\n", role_label, entry.content));
            }
            user_prompt.push_str("</historial_chat>\n\n");
        }
    }

    user_prompt.push_str(&format!("PREGUNTA ACTUAL DEL USUARIO:\n{}", message.trim()));

    // 3. Llamar al LLM. Reusa SHARED_CLIENT (ya tiene connection pooling)
    // en lugar de crear Client nuevo por request (antes: 20-50ms overhead).
    let client = &*SHARED_CLIENT;

    let start = std::time::Instant::now();

    let response = generate_summary(
        client,
        &LLMProvider::Ollama,
        &model_to_use,
        "",
        CHAT_SYSTEM_PROMPT,
        &user_prompt,
        None,
        None,
        Some(500),  // chat puede ser un poco más largo
        Some(0.5),  // temperatura media: balance entre creatividad y consistencia
        Some(0.9),
        None,
        None,
    )
    .await
    .map_err(|e| format!("Error LLM chat: {}", e))?;

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(ChatResponse {
        answer: response.trim().to_string(),
        model: model_to_use,
        latency_ms,
        context_chars: context.char_count,
        context_turns: context.turn_count,
        user_turns: context.user_turns,
        interlocutor_turns: context.interlocutor_turns,
    })
}

/// Evento de streaming: token del assistant mientras Ollama genera.
#[derive(Debug, Serialize, Clone)]
pub struct ChatStreamToken {
    /// stream_id del request (para demultiplexar si hay múltiples streams).
    pub stream_id: String,
    /// Texto del chunk (puede ser 1+ tokens).
    pub delta: String,
    /// Si es el último chunk.
    pub done: bool,
}

/// Evento final: metadata completa tras el último token.
#[derive(Debug, Serialize, Clone)]
pub struct ChatStreamComplete {
    pub stream_id: String,
    pub model: String,
    pub latency_ms: u64,
    pub first_token_ms: u64,
    pub total_tokens: usize,
    pub context_chars: usize,
}

/// Comando Tauri: chat con streaming real de Ollama.
///
/// Emite eventos:
/// - `coach-chat-token` { stream_id, delta, done } por cada chunk
/// - `coach-chat-complete` { stream_id, model, latency_ms, first_token_ms } al final
/// - `coach-chat-error` { stream_id, error } si falla
///
/// Retorna inmediatamente con stream_id; el frontend escucha los eventos.
#[tauri::command]
pub async fn coach_chat_stream(
    app: tauri::AppHandle,
    message: String,
    meeting_id: Option<String>,
    live_transcript: Option<String>,
    history: Option<Vec<ChatHistoryEntry>>,
    model: Option<String>,
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Mensaje vacío".to_string());
    }

    let stream_id = format!("chat-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros()).unwrap_or(0));
    let model_to_use = model.unwrap_or_else(|| crate::coach::prompt::DEFAULT_MODEL.to_string());

    // Construir contexto + user_prompt (misma lógica que coach_chat sync)
    let context = if let Some(live) = live_transcript.filter(|s| !s.trim().is_empty()) {
        let live_len = live.len();
        crate::coach::context::CoachContext {
            formatted: live,
            turn_count: 0,
            char_count: live_len,
            user_turns: 0,
            interlocutor_turns: 0,
        }
    } else if let Some(mid) = meeting_id.as_ref() {
        let state = app.try_state::<crate::state::AppState>()
            .ok_or_else(|| "AppState no disponible".to_string())?;
        let pool = state.db_manager.pool();
        build_context(pool, mid, ContextMode::Recent { max_chars: CHAT_CONTEXT_MAX_CHARS })
            .await
            .unwrap_or_else(|_| crate::coach::context::CoachContext::empty())
    } else {
        crate::coach::context::CoachContext::empty()
    };

    let mut user_prompt = String::new();
    if !context.is_empty() {
        user_prompt.push_str(&format!(
            "<transcripcion turnos=\"{}\">\n{}\n</transcripcion>\n\n",
            context.turn_count, context.formatted
        ));
    }
    if let Some(hist) = history {
        if !hist.is_empty() {
            user_prompt.push_str("<historial_chat>\n");
            for entry in hist.iter().take(20) {
                let role_label = if entry.role == "user" { "Usuario" } else { "Coach" };
                user_prompt.push_str(&format!("{}: {}\n", role_label, entry.content));
            }
            user_prompt.push_str("</historial_chat>\n\n");
        }
    }
    user_prompt.push_str(&format!("PREGUNTA ACTUAL DEL USUARIO:\n{}", message.trim()));

    let context_chars = context.char_count;
    let sid = stream_id.clone();
    let app_handle = app.clone();

    // Spawn tarea para streaming sin bloquear el comando
    tauri::async_runtime::spawn(async move {
        let client = &*SHARED_CLIENT;
        let body = serde_json::json!({
            "model": model_to_use,
            "messages": [
                {"role": "system", "content": CHAT_SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt}
            ],
            "stream": true,
            "keep_alive": -1,
            "options": {
                "num_gpu": -1,
                "num_thread": 4,
                "num_ctx": 4096,
                "num_predict": 220,
                "temperature": 0.5,
                "top_p": 0.9,
                "flash_attention": true
            }
        });

        let start = std::time::Instant::now();
        let mut first_token_ms: Option<u64> = None;
        let mut total_tokens = 0usize;

        let response = match client
            .post("http://localhost:11434/api/chat")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = app_handle.emit(
                    "coach-chat-error",
                    serde_json::json!({ "stream_id": sid, "error": e.to_string() }),
                );
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let _ = app_handle.emit(
                "coach-chat-error",
                serde_json::json!({ "stream_id": sid, "error": format!("HTTP {}", status) }),
            );
            return;
        }

        // Ollama /api/chat stream: cada línea es un JSON con { message: { content }, done }
        let mut stream = response.bytes_stream();
        let mut buf = Vec::<u8>::new();

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => {
                    let _ = app_handle.emit(
                        "coach-chat-error",
                        serde_json::json!({ "stream_id": sid, "error": e.to_string() }),
                    );
                    return;
                }
            };
            buf.extend_from_slice(&bytes);

            // Procesar líneas completas (NDJSON)
            while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=nl_pos).collect();
                let line_str = String::from_utf8_lossy(&line[..line.len() - 1]);
                if line_str.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = match serde_json::from_str(&line_str) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let delta = value
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                let done = value.get("done").and_then(|d| d.as_bool()).unwrap_or(false);

                if !delta.is_empty() {
                    if first_token_ms.is_none() {
                        first_token_ms = Some(start.elapsed().as_millis() as u64);
                    }
                    total_tokens += 1;
                    let _ = app_handle.emit(
                        "coach-chat-token",
                        ChatStreamToken {
                            stream_id: sid.clone(),
                            delta,
                            done: false,
                        },
                    );
                }

                if done {
                    let latency_ms = start.elapsed().as_millis() as u64;
                    let _ = app_handle.emit(
                        "coach-chat-token",
                        ChatStreamToken {
                            stream_id: sid.clone(),
                            delta: String::new(),
                            done: true,
                        },
                    );
                    let _ = app_handle.emit(
                        "coach-chat-complete",
                        ChatStreamComplete {
                            stream_id: sid.clone(),
                            model: model_to_use.clone(),
                            latency_ms,
                            first_token_ms: first_token_ms.unwrap_or(latency_ms),
                            total_tokens,
                            context_chars,
                        },
                    );
                    return;
                }
            }
        }

        // Si el stream terminó sin un `done: true`, emitir complete igualmente.
        let latency_ms = start.elapsed().as_millis() as u64;
        let _ = app_handle.emit(
            "coach-chat-complete",
            ChatStreamComplete {
                stream_id: sid.clone(),
                model: model_to_use.clone(),
                latency_ms,
                first_token_ms: first_token_ms.unwrap_or(latency_ms),
                total_tokens,
                context_chars,
            },
        );
    });

    Ok(stream_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_response_struct() {
        let r = ChatResponse {
            answer: "Test".to_string(),
            model: "gemma4:latest".to_string(),
            latency_ms: 1234,
            context_chars: 100,
            context_turns: 5,
            user_turns: 2,
            interlocutor_turns: 3,
        };
        assert_eq!(r.user_turns + r.interlocutor_turns, r.context_turns);
    }

    #[test]
    fn test_history_entry_deserialize() {
        let json = r#"{"role":"user","content":"Hola"}"#;
        let entry: ChatHistoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.role, "user");
        assert_eq!(entry.content, "Hola");
    }
}
