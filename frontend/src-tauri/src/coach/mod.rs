//! Módulo Coach — Copiloto IA en tiempo real para reuniones.
//!
//! Recibe ventanas de transcripción acumulada y genera sugerencias cortas
//! (1-2 oraciones) accionables para el usuario durante la conversación.
//!
//! 100% local: solo proveedor Ollama permitido. Reusa `summary::llm_client`.

pub mod bookmarks;
pub mod chat;
pub mod commands;
pub mod context;
pub mod evaluation_types;
pub mod evaluator;
pub mod floating;
pub mod meeting_chat;
pub mod meeting_type;
pub mod nudge_engine;
pub mod prompt;
pub mod prompts;
pub mod trigger;
#[cfg(test)]
mod stress_tests;
#[cfg(test)]
mod perf_tests;

pub use bookmarks::{coach_add_bookmark, coach_delete_bookmark, coach_get_bookmarks, Bookmark};
pub use chat::{coach_chat, ChatResponse};
pub use commands::{
    coach_get_status, coach_set_model, coach_suggest, CoachStatus, CoachSuggestion,
};
pub use context::{build_context, CoachContext, ContextMode};
pub use evaluator::{
    coach_evaluate_communication, CommunicationFeedback, CommunicationObservations,
};
pub use nudge_engine::coach_evaluate_nudge;
pub use trigger::{analyze_turn, coach_analyze_trigger, TriggerCategory, TriggerSignal};
