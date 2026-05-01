//! Gestión del estado de modelos Ollama y cliente HTTP compartido.
//!
//! Mantiene tres modelos configurables: tips, evaluación y chat.
//! Proporciona cliente HTTP reutilizable para evitar cold-start por request.

use crate::coach::prompt::DEFAULT_MODEL;
use reqwest::Client;
use std::sync::atomic::AtomicU64;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

/// Modelo activo (mutable, defaulteado a Phi-3.5).
pub static CURRENT_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(DEFAULT_MODEL.to_string()));

/// Modelo activo para evaluación post-meeting (configurable por usuario).
/// Default `gemma3:4b` para compatibilidad con laptops 8GB RAM.
pub static EVALUATION_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("gemma3:4b".to_string()));

/// Modelo activo para chat con reuniones (configurable por usuario).
pub static CHAT_MODEL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new("gemma3:4b".to_string()));

/// Latencia del último request (ms). 0 = aún no medido.
pub static LAST_LATENCY_MS: AtomicU64 = AtomicU64::new(0);

/// Shared HTTP client for Ollama requests (eliminates cold-start per-request overhead).
/// HTTP client compartido entre coach_suggest y coach_chat.
/// Timeout 60s para chat (respuestas más largas); pool reutiliza conexiones TCP
/// a localhost:11434 → elimina 20-50ms de setup por request.
pub static SHARED_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(4)
        .build()
        .expect("Failed to create shared HTTP client for Ollama")
});

/// Get current model without locking (for startup warm-up).
pub fn get_current_model() -> Result<String, String> {
    CURRENT_MODEL
        .lock()
        .map(|g| g.clone())
        .map_err(|e| format!("Failed to get current model: {}", e))
}

/// Health check rápido a Ollama (timeout 2s).
pub async fn check_ollama_running() -> bool {
    let client = match Client::builder().timeout(Duration::from_secs(2)).build() {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
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
