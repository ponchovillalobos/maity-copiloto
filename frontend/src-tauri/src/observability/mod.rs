//! Módulo de observabilidad para el Dashboard milimétrico (`/dashboard`).
//!
//! Expone:
//! - System metrics live (CPU/RAM cada 1s) via evento `system-metrics`
//! - LLM throughput live (tok/s del modelo actual) via evento `llm-throughput`
//! - Tabla `dev_iterations` con histórico de cada test cargado en `/dev`
//! - Tabla `button_audit` con matriz manual de estado de cada botón
//!
//! Dependencias: `sysinfo` (ya en Cargo.toml), sqlx, tauri.

pub mod button_audit;
pub mod iteration_log;
pub mod system_monitor;
pub mod timing;

pub use button_audit::{
    dashboard_list_buttons, dashboard_seed_buttons, dashboard_update_button_status, ButtonRow,
};
pub use iteration_log::{
    dashboard_get_iteration_detail, dashboard_get_summary, dashboard_list_iterations,
    DashboardSummary, IterationDetail, IterationRow, NewIterationRecord,
};
pub use system_monitor::{run_system_monitor, SystemMetrics};
pub use timing::Timer;
