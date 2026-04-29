use reqwest::Client;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

/// LLM Provider enumeration (local providers only)
#[derive(Debug, Clone, PartialEq)]
pub enum LLMProvider {
    Ollama,
    BuiltInAI,
}

impl LLMProvider {
    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Self::Ollama),
            "builtin-ai" | "local-llama" | "localllama" => Ok(Self::BuiltInAI),
            _ => Err(format!("Unsupported LLM provider: {}", s)),
        }
    }
}

/// Generates a summary using the specified LLM provider
///
/// # Arguments
/// * `client` - Reqwest HTTP client (reused for performance)
/// * `provider` - The LLM provider to use
/// * `model_name` - The specific model to use (e.g., "gpt-4", "claude-3-opus")
/// * `api_key` - API key for the provider (not needed for Ollama)
/// * `system_prompt` - System instructions for the LLM
/// * `user_prompt` - User query/content to process
/// * `ollama_endpoint` - Optional custom Ollama endpoint (defaults to localhost:11434)
/// * `custom_openai_endpoint` - Optional custom OpenAI-compatible endpoint
/// * `max_tokens` - Optional max tokens (for CustomOpenAI provider)
/// * `temperature` - Optional temperature (for CustomOpenAI provider)
/// * `top_p` - Optional top_p (for CustomOpenAI provider)
/// * `app_data_dir` - Optional app data directory (for BuiltInAI provider)
/// * `cancellation_token` - Optional token to cancel the request
///
/// # Returns
/// The generated summary text or an error message
pub async fn generate_summary(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    ollama_endpoint: Option<&str>,
    _custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<String, String> {
    // Check if cancelled before starting
    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }

    // Handle BuiltInAI provider separately (uses local sidecar, no HTTP API)
    if provider == &LLMProvider::BuiltInAI {
        let app_data_dir = app_data_dir
            .ok_or_else(|| "app_data_dir is required for BuiltInAI provider".to_string())?;

        return crate::summary::summary_engine::generate_with_builtin(
            app_data_dir,
            model_name,
            system_prompt,
            user_prompt,
            cancellation_token,
        )
        .await
        .map_err(|e| e.to_string());
    }

    // Cloud providers no soportados (app es 100% local).
    let _ = api_key;
    let _ = ollama_endpoint;
    let _ = max_tokens;
    let _ = temperature;
    let _ = top_p;
    Err(format!(
        "Provider {:?} no soportado en builds locales (solo BuiltInAI/Ollama)",
        provider
    ))
}

/// Streaming variant for BuiltInAI: yields token chunks as they're generated.
///
/// Returns a receiver that emits `Ok(text)` per token, ends with channel close on
/// success, or sends `Err(msg)` and closes if generation fails.
///
/// Only supports `LLMProvider::BuiltInAI`. For Ollama, fall back to non-streaming
/// `generate_summary` (Ollama HTTP streaming was removed when migrating to sidecar).
pub async fn generate_summary_stream(
    provider: &LLMProvider,
    model_name: &str,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: Option<u32>,
    app_data_dir: Option<&PathBuf>,
) -> Result<tokio::sync::mpsc::Receiver<Result<String, String>>, String> {
    if provider != &LLMProvider::BuiltInAI {
        return Err("generate_summary_stream solo soporta BuiltInAI por ahora".to_string());
    }

    let app_data_dir = app_data_dir
        .ok_or_else(|| "app_data_dir is required for BuiltInAI provider".to_string())?;

    crate::summary::summary_engine::generate_stream_with_builtin(
        app_data_dir,
        model_name,
        system_prompt,
        user_prompt,
        max_tokens.map(|n| n as i32),
    )
    .await
    .map_err(|e| e.to_string())
}

