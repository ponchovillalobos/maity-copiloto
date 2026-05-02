//! Tipos del Coach: CoachSuggestion, CoachStatus, CoachModelsConfig, RawSuggestion.

use serde::{Deserialize, Serialize};

/// Sugerencia de coaching que se retorna al frontend.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoachSuggestion {
    pub tip: String,
    #[serde(default = "default_category")]
    pub category: String,
    /// Subcategoría específica de la técnica (ej: "spin_problem_to_implication").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// Framework de origen (ej: "SPIN", "Chris Voss", "Cialdini").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique: Option<String>,
    /// Nivel de prioridad: "critical" | "important" | "soft".
    /// Se deriva de confidence si el LLM no la provee.
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// V3.1: tipo de tip — "recognition"|"observation"|"corrective"|"introspective".
    /// Se infiere si el LLM no lo provee (fallback).
    #[serde(default = "default_tip_type")]
    pub tip_type: String,
    pub timestamp: i64,
    pub model: String,
    pub latency_ms: u64,
    /// BUG #15 fix: id de la fila en `coach_tips_log` (Some en catch-up vía DB,
    /// None en sugerencia live recién generada). Permite a la burbuja flotante
    /// pollear `coach_get_recent_tips` y filtrar por id > lastSeenId.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
}

/// Estado del coach: modelo activo, Ollama running, latencia.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoachStatus {
    pub model: String,
    pub ollama_running: bool,
    pub last_latency_ms: u64,
}

/// Configuración de los 3 modelos del coach (tips/evaluación/chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoachModelsConfig {
    pub tips_model: String,
    pub evaluation_model: String,
    pub chat_model: String,
}

/// Salida cruda esperada del LLM (JSON dentro del content).
#[derive(Debug, Deserialize)]
pub struct RawSuggestion {
    pub tip: String,
    #[serde(default = "default_category")]
    pub category: String,
    /// V3.1 nuevo: tipo de tip (opcional, se infiere si falta).
    #[serde(default)]
    pub tip_type: Option<String>,
    #[serde(default)]
    pub subcategory: Option<String>,
    #[serde(default)]
    pub technique: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

// Default functions for serde
pub fn default_priority() -> String {
    "soft".to_string()
}

pub fn default_confidence() -> f32 {
    0.7
}

pub fn default_category() -> String {
    "general".to_string()
}

pub fn default_tip_type() -> String {
    "observation".to_string()
}
