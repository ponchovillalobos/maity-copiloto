//! Task tokio que muestrea CPU/RAM cada 1s y emite `system-metrics` al frontend.
//! v23: además escribe snapshot a `%APPDATA%/com.maity.ai/runtime.json`
//! para que dashboard-web (proceso separado, sin acceso Tauri) pueda leerlo.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub ts: i64,
    pub cpu_pct: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub process_cpu_pct: f32,
    pub process_ram_mb: u64,
    pub thread_count: usize,
    /// v23: flag is_recording leído del estado global.
    pub is_recording: bool,
    /// v26: RAM consumida por el llama-helper sidecar (LLM coach + eval).
    /// Separada del proceso Maity para diagnosticar costo del modelo.
    pub model_ram_mb: u64,
    /// v26: CPU del modelo (llama-helper)
    pub model_cpu_pct: f32,
    /// v26: total efectivo Maity (app + modelo) en MB.
    pub maity_total_ram_mb: u64,
}

/// Loop infinito: cada 1s muestrea sysinfo y emite evento. Se cancela
/// automáticamente cuando el AppHandle se drop (al cerrar app).
pub async fn run_system_monitor<R: Runtime>(app: AppHandle<R>) {
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything()),
    );

    let pid = sysinfo::get_current_pid().ok();

    sys.refresh_cpu_all();
    tokio::time::sleep(Duration::from_millis(250)).await;

    let runtime_path = app.path().app_data_dir().ok().map(|p| p.join("runtime.json"));

    loop {
        sys.refresh_cpu_all();
        sys.refresh_memory();
        // v26: refrescar TODOS los procesos para encontrar llama-helper
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let cpu_pct = sys.global_cpu_usage();
        let ram_used_mb = sys.used_memory() / (1024 * 1024);
        let ram_total_mb = sys.total_memory() / (1024 * 1024);

        let (process_cpu_pct, process_ram_mb, thread_count) = if let Some(p) = pid {
            if let Some(proc) = sys.process(p) {
                (
                    proc.cpu_usage(),
                    proc.memory() / (1024 * 1024),
                    proc.tasks().map(|t| t.len()).unwrap_or(1),
                )
            } else {
                (0.0, 0, 1)
            }
        } else {
            (0.0, 0, 1)
        };

        // v26: buscar llama-helper sidecar para medir RAM del modelo.
        let mut model_ram_mb = 0u64;
        let mut model_cpu_pct = 0.0f32;
        for proc in sys.processes().values() {
            let name = proc.name().to_string_lossy().to_lowercase();
            if name.contains("llama-helper") {
                model_ram_mb += proc.memory() / (1024 * 1024);
                model_cpu_pct += proc.cpu_usage();
            }
        }
        let maity_total_ram_mb = process_ram_mb + model_ram_mb;

        let is_recording = crate::audio::recording_commands::is_recording().await;

        let metrics = SystemMetrics {
            ts: chrono::Utc::now().timestamp_millis(),
            cpu_pct,
            ram_used_mb,
            ram_total_mb,
            process_cpu_pct,
            process_ram_mb,
            thread_count,
            is_recording,
            model_ram_mb,
            model_cpu_pct,
            maity_total_ram_mb,
        };

        let _ = app.emit("system-metrics", metrics.clone());

        // v23: persistir snapshot para que dashboard-web standalone lo lea.
        if let Some(ref path) = runtime_path {
            if let Ok(json) = serde_json::to_string(&metrics) {
                let _ = std::fs::write(path, json);
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
