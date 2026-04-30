//! Task tokio que muestrea CPU/RAM cada 1s y emite `system-metrics` al frontend.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use sysinfo::{CpuRefreshKind, RefreshKind, System};
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub ts: i64,
    pub cpu_pct: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub process_cpu_pct: f32,
    pub process_ram_mb: u64,
    pub thread_count: usize,
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

    loop {
        sys.refresh_cpu_all();
        sys.refresh_memory();
        if let Some(p) = pid {
            sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[p]), true);
        }

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

        let metrics = SystemMetrics {
            ts: chrono::Utc::now().timestamp_millis(),
            cpu_pct,
            ram_used_mb,
            ram_total_mb,
            process_cpu_pct,
            process_ram_mb,
            thread_count,
        };

        let _ = app.emit("system-metrics", metrics);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
