//! Auto-setup: asegura que todas las dependencias estén listas al arrancar
//! la app SIN intervención del usuario.
//!
//! Verifica en orden:
//!   1. Ollama corriendo (localhost:11434)
//!   2. Modelo LLM default disponible (gemma3:4b) — descarga si falta
//!   3. Modelo Parakeet ONNX disponible — descarga si falta
//!
//! Emite evento `auto-setup-progress` con el estado de cada paso para que
//! el frontend muestre un overlay con progreso.
//!
//! El proceso corre EN BACKGROUND después de que la UI carga — no bloquea
//! el startup. Si Ollama no está instalado, emite un evento pidiendo al
//! usuario instalarlo (con link directo), pero el resto de la app
//! sigue funcional (grabación + transcripción con Parakeet).

use log::{info, warn};
use serde::Serialize;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

/// Modelo LLM default para el coach (descarga automática si falta).
/// Modelo pequeño y rápido (4B params, 3.3GB, 39 tok/s warm, 1.7-2.7s por respuesta).
/// Benchmark 2026-04-15: 3x más rápido que gemma4:latest.
const DEFAULT_LLM_MODEL: &str = "gemma3:4b";
/// Modelo Parakeet default.
const DEFAULT_PARAKEET_MODEL: &str = "parakeet-tdt-0.6b-v3-int8";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AutoSetupProgress {
    /// "checking" | "ollama_missing" | "pulling_llm" | "downloading_parakeet" | "done" | "error"
    pub phase: String,
    /// Paso actual (1-3).
    pub step: u8,
    /// Total de pasos.
    pub total_steps: u8,
    /// Mensaje humano para mostrar al usuario.
    pub message: String,
    /// Si hay progreso descargable (0-100) o None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u32>,
    /// Recurso siendo procesado (nombre del modelo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
}

impl AutoSetupProgress {
    fn new(phase: &str, step: u8, message: &str) -> Self {
        Self {
            phase: phase.to_string(),
            step,
            total_steps: 3,
            message: message.to_string(),
            percent: None,
            resource: None,
        }
    }
}

fn emit<R: Runtime>(app: &AppHandle<R>, progress: AutoSetupProgress) {
    if let Err(e) = app.emit("auto-setup-progress", progress) {
        warn!("Failed to emit auto-setup-progress: {}", e);
    }
}

/// Entrada principal: corre en background tras el startup.
pub async fn run<R: Runtime>(app: AppHandle<R>) {
    info!("[auto-setup] Starting dependency check...");
    emit(&app, AutoSetupProgress::new("checking", 1, "Verificando Ollama..."));

    // --- PASO 1: Verificar Ollama ---
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("[auto-setup] Failed to build HTTP client: {}", e);
            emit(&app, AutoSetupProgress::new("error", 1, "Error creando cliente HTTP"));
            return;
        }
    };

    let ollama_ok = match client.get("http://localhost:11434/api/tags").send().await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    };

    if !ollama_ok {
        warn!("[auto-setup] Ollama no responde en localhost:11434");
        emit(
            &app,
            AutoSetupProgress {
                phase: "ollama_missing".to_string(),
                step: 1,
                total_steps: 3,
                message: "Ollama no detectado. Instálalo desde https://ollama.com para habilitar el coach IA.".to_string(),
                percent: None,
                resource: Some("ollama".to_string()),
            },
        );
        // No abortamos: seguimos con Parakeet (grabación funciona sin Ollama).
    } else {
        info!("[auto-setup] Ollama OK");

        // --- PASO 2: Verificar modelo LLM ---
        emit(
            &app,
            AutoSetupProgress {
                phase: "checking".to_string(),
                step: 2,
                total_steps: 3,
                message: format!("Verificando modelo {}...", DEFAULT_LLM_MODEL),
                percent: None,
                resource: Some(DEFAULT_LLM_MODEL.to_string()),
            },
        );

        let models_list = match client.get("http://localhost:11434/api/tags").send().await {
            Ok(r) => r.json::<serde_json::Value>().await.ok(),
            Err(_) => None,
        };

        let model_present = models_list
            .as_ref()
            .and_then(|v| v.get("models"))
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter().any(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s == DEFAULT_LLM_MODEL || s.starts_with(&format!("{}:", DEFAULT_LLM_MODEL.split(':').next().unwrap_or(""))))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if !model_present {
            info!("[auto-setup] Pulling {} via Ollama...", DEFAULT_LLM_MODEL);
            emit(
                &app,
                AutoSetupProgress {
                    phase: "pulling_llm".to_string(),
                    step: 2,
                    total_steps: 3,
                    message: format!("Descargando modelo de IA ({}) — primera vez, ~2GB...", DEFAULT_LLM_MODEL),
                    percent: Some(0),
                    resource: Some(DEFAULT_LLM_MODEL.to_string()),
                },
            );

            if let Err(e) = crate::ollama::ollama::pull_ollama_model(
                app.clone(),
                DEFAULT_LLM_MODEL.to_string(),
                None,
            )
            .await
            {
                warn!("[auto-setup] Pull LLM model failed: {}", e);
                emit(
                    &app,
                    AutoSetupProgress {
                        phase: "error".to_string(),
                        step: 2,
                        total_steps: 3,
                        message: format!("Error descargando modelo: {}", e),
                        percent: None,
                        resource: Some(DEFAULT_LLM_MODEL.to_string()),
                    },
                );
            } else {
                info!("[auto-setup] LLM model ready: {}", DEFAULT_LLM_MODEL);
            }
        } else {
            info!("[auto-setup] LLM model {} already present", DEFAULT_LLM_MODEL);
        }
    }

    // --- PASO 3: Verificar Parakeet ONNX ---
    emit(
        &app,
        AutoSetupProgress {
            phase: "checking".to_string(),
            step: 3,
            total_steps: 3,
            message: format!("Verificando modelo de transcripción ({})...", DEFAULT_PARAKEET_MODEL),
            percent: None,
            resource: Some(DEFAULT_PARAKEET_MODEL.to_string()),
        },
    );

    // parakeet_validate_model_ready_with_config se encarga de descargar si falta
    // y emite eventos parakeet-model-download-progress.
    let preloaded = crate::audio::transcription::engine::PRELOADED_ENGINE
        .read()
        .ok()
        .and_then(|g| g.clone());

    if preloaded.as_ref().map(|(p, _)| p.as_str()) == Some("parakeet") {
        info!("[auto-setup] Parakeet ya precargado");
    } else {
        emit(
            &app,
            AutoSetupProgress {
                phase: "downloading_parakeet".to_string(),
                step: 3,
                total_steps: 3,
                message: "Descargando modelo de transcripción (~670MB)...".to_string(),
                percent: Some(0),
                resource: Some(DEFAULT_PARAKEET_MODEL.to_string()),
            },
        );

        if let Err(e) = crate::parakeet_engine::commands::parakeet_validate_model_ready_with_config(&app).await {
            warn!("[auto-setup] Parakeet validation failed: {}", e);
            emit(
                &app,
                AutoSetupProgress {
                    phase: "error".to_string(),
                    step: 3,
                    total_steps: 3,
                    message: format!("Error modelo transcripción: {}", e),
                    percent: None,
                    resource: Some(DEFAULT_PARAKEET_MODEL.to_string()),
                },
            );
            return;
        }
    }

    info!("[auto-setup] All dependencies ready");
    emit(
        &app,
        AutoSetupProgress {
            phase: "done".to_string(),
            step: 3,
            total_steps: 3,
            message: "Listo para grabar".to_string(),
            percent: Some(100),
            resource: None,
        },
    );
}

/// Comando Tauri para re-ejecutar auto-setup desde el frontend (ej. botón "Reintentar").
#[tauri::command]
pub async fn auto_setup_retry<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        run(app).await;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_serialize_includes_camel_case() {
        let p = AutoSetupProgress {
            phase: "pulling_llm".to_string(),
            step: 2,
            total_steps: 3,
            message: "test".to_string(),
            percent: Some(50),
            resource: Some("gemma4".to_string()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"totalSteps\":3"));
        assert!(json.contains("\"phase\":\"pulling_llm\""));
        assert!(json.contains("\"percent\":50"));
    }

    #[test]
    fn test_progress_omits_optional_fields() {
        let p = AutoSetupProgress::new("done", 3, "Listo");
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("percent"));
        assert!(!json.contains("resource"));
    }
}
