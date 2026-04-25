//! Ventana flotante always-on-top para mostrar tips del coach durante video llamadas.
//!
//! Crea una WebviewWindow independiente apuntando a `/floating`. Glass-morphic,
//! transparente, sin decoración. Reusa eventos `coach-tip-update` ya emitidos
//! desde `coach::commands` para sincronizar tips en tiempo real.

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

const FLOATING_LABEL: &str = "coach-floating";
const DEFAULT_WIDTH: f64 = 320.0;
const DEFAULT_HEIGHT: f64 = 380.0;

/// Abre la ventana flotante. Si ya existe, la enfoca y la muestra.
#[tauri::command]
pub async fn open_floating_coach<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(FLOATING_LABEL) {
        existing.show().map_err(|e| e.to_string())?;
        existing.set_always_on_top(true).map_err(|e| e.to_string())?;
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(&app, FLOATING_LABEL, WebviewUrl::App("floating".into()))
        .title("Maity Coach")
        .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .min_inner_size(240.0, 200.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(true)
        .skip_taskbar(false)
        .visible(true)
        .build()
        .map_err(|e| format!("No se pudo crear ventana flotante: {}", e))?;

    if let Some(monitor) = window.primary_monitor().ok().flatten() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let mon_w = size.width as f64 / scale;
        let mon_h = size.height as f64 / scale;
        let target_x = (mon_w - DEFAULT_WIDTH - 32.0).max(0.0);
        let target_y = 80.0_f64.min((mon_h - DEFAULT_HEIGHT - 32.0).max(0.0));
        window
            .set_position(LogicalPosition::new(target_x, target_y))
            .map_err(|e| e.to_string())?;
        window
            .set_size(LogicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Cierra la ventana flotante si está activa.
#[tauri::command]
pub async fn close_floating_coach<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(FLOATING_LABEL) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Cambia entre modo compacto (140x100) y modo expandido (320x380).
#[tauri::command]
pub async fn floating_toggle_compact<R: Runtime>(
    app: AppHandle<R>,
    compact: bool,
) -> Result<(), String> {
    let window = app
        .get_webview_window(FLOATING_LABEL)
        .ok_or_else(|| "Ventana flotante no abierta".to_string())?;

    let (w, h) = if compact { (140.0, 110.0) } else { (DEFAULT_WIDTH, DEFAULT_HEIGHT) };
    window
        .set_size(LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    Ok(())
}
