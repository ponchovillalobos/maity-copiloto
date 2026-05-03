//! Gestión del estado de modelos del runtime local integrado (BuiltInAI sidecar).
//!
//! v31.22: modelo unificado qwen3:1.7b para tips/eval/chat. Sin defaults gemma.

use crate::coach::prompt::DEFAULT_MODEL;
use reqwest::Client;
use std::sync::atomic::AtomicU64;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// Modelo activo para tips coach (qwen3:1.7b).
pub static CURRENT_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(DEFAULT_MODEL.to_string()));

/// Modelo activo para evaluación post-meeting. v31.22: unificado a qwen3:1.7b.
pub static EVALUATION_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(DEFAULT_MODEL.to_string()));

/// Modelo activo para chat con reuniones. v31.22: unificado a qwen3:1.7b.
pub static CHAT_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(DEFAULT_MODEL.to_string()));

/// Latencia del último request (ms). 0 = aún no medido.
pub static LAST_LATENCY_MS: AtomicU64 = AtomicU64::new(0);

/// HTTP client compartido — usado por coach_chat. coach_simple_tick (BuiltInAI)
/// lo ignora pero el shared pool evita cold-start si otro path lo usa.
pub static SHARED_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(4)
        .build()
        .expect("Failed to create shared HTTP client")
});

/// Get current model without locking (for startup warm-up).
pub fn get_current_model() -> Result<String, String> {
    CURRENT_MODEL
        .lock()
        .map(|g| g.clone())
        .map_err(|e| format!("Failed to get current model: {}", e))
}

/// Health check del runtime local integrado. Renombrado v31.22 a `ai_ready`.
/// Mantiene alias `check_ollama_running` para compat con imports existentes.
pub async fn ai_ready() -> bool {
    crate::summary::summary_engine::is_sidecar_healthy().await
}

/// Compat alias — eliminar después de actualizar todos los callers.
pub async fn check_ollama_running() -> bool {
    ai_ready().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_model_returns_default() {
        let model = get_current_model().unwrap();
        assert!(!model.is_empty());
    }
}
