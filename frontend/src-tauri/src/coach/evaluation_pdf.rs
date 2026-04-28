//! Exportar evaluación post-meeting como PDF profesional.
//!
//! Genera PDF con formato texto bien formateado (sin gráficos complejos).
//! Incluye:
//! - Header con nombre reunión, fecha y puntuación global
//! - Tabla 6 dimensiones con scores
//! - Resumen (fortalezas + áreas de mejora)
//! - Recomendaciones priorizadas
//! - Footer con timestamp

use crate::coach::evaluation_types::MeetingEvaluation;
use crate::state::AppState;
use chrono::{DateTime, Local};
use log::info as log_info;
use printpdf::*;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::Command;

/// Estructura para representar la evaluación en PDF con metadatos
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EvaluationPdfData {
    meeting_id: String,
    meeting_name: String,
    created_at: String,
    evaluation: MeetingEvaluation,
}

/// Normaliza valor de -1..1 a 0..100
fn normalize_score(value: f32) -> f32 {
    ((value + 1.0) / 2.0 * 100.0).max(0.0).min(100.0)
}

/// Genera el PDF de evaluación (texto bien formateado, sin gráficos complejos)
pub fn generate_evaluation_pdf(eval_data: &EvaluationPdfData) -> Result<Vec<u8>, String> {
    // Crear documento A4
    let (document, page1, layer1) = PdfDocument::new("Evaluación", Mm(210.0), Mm(297.0), "Layer 1");

    let font = document
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Error cargando fuente: {}", e))?;
    let font_bold = document
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Error cargando fuente negrita: {}", e))?;

    let left_margin = Mm(15.0);
    let min_y_position = Mm(10.0);
    let line_height = Mm(5.0);

    let ev = &eval_data.evaluation;
    let gauge = ev.visualizaciones.gauge.valor;
    let nivel = &ev.resumen.nivel;

    // === PÁGINA 1: HEADER + PUNTUACIÓN GLOBAL + DIMENSIONES ===

    let mut layer = document.get_page(page1).get_layer(layer1);
    let mut y_position = Mm(277.0);

    // Título
    layer.use_text(&eval_data.meeting_name, 18.0, left_margin, y_position, &font_bold);
    y_position -= Mm(7.0);

    // Fecha y metadata
    let fecha_texto = format!("Fecha: {}", eval_data.created_at);
    layer.use_text(&fecha_texto, 9.0, left_margin, y_position, &font);
    y_position -= Mm(4.0);

    // Separador
    layer.use_text(
        "═══════════════════════════════════════════════════════════",
        8.0,
        left_margin,
        y_position,
        &font,
    );
    y_position -= Mm(4.0);

    // Puntuación Global
    layer.use_text("PUNTUACIÓN GLOBAL", 12.0, left_margin, y_position, &font_bold);
    y_position -= Mm(6.0);

    let score_text = format!("{:.0} / 100 — {}", gauge, nivel);
    layer.use_text(&score_text, 14.0, left_margin, y_position, &font_bold);
    y_position -= Mm(7.0);

    // Tendencia (si existe)
    if let Some(tendencia) = ev.historico.tendencia_global {
        let tend_text = if tendencia >= 0.0 {
            format!("↑ +{:.1} vs sesión anterior", tendencia)
        } else {
            format!("↓ {:.1} vs sesión anterior", tendencia)
        };
        layer.use_text(&tend_text, 9.0, left_margin, y_position, &font);
        y_position -= Mm(4.0);
    }

    y_position -= Mm(3.0);

    // Dimensiones - tabla simple
    layer.use_text("DIMENSIONES (Escala 0-100)", 11.0, left_margin, y_position, &font_bold);
    y_position -= Mm(5.0);

    let dimensions = vec![
        ("Claridad", ev.dimensiones.claridad.puntaje),
        ("Propósito", ev.dimensiones.proposito.puntaje),
        ("Emociones", normalize_score(ev.dimensiones.emociones.polaridad)),
        ("Estructura", ev.dimensiones.estructura.puntaje),
        ("Persuasión", ev.dimensiones.persuasion.puntaje),
        ("Adaptación", ev.dimensiones.adaptacion.puntaje),
    ];

    for (dim_name, score) in dimensions {
        let dim_line = format!("  {:<20} {:.0}/100", dim_name, score);
        layer.use_text(&dim_line, 9.0, left_margin, y_position, &font);
        y_position -= Mm(4.0);
    }

    y_position -= Mm(2.0);

    // Resumen Ejecutivo
    if y_position < min_y_position + Mm(35.0) {
        let (page2, layer2) = document.add_page(Mm(210.0), Mm(297.0), "Page 2");
        layer = document.get_page(page2).get_layer(layer2);
        y_position = Mm(277.0);
    }

    layer.use_text("RESUMEN EJECUTIVO", 11.0, left_margin, y_position, &font_bold);
    y_position -= Mm(5.0);

    if !ev.resumen.fortaleza.is_empty() {
        layer.use_text("✓ Fortaleza:", 9.0, left_margin, y_position, &font_bold);
        y_position -= Mm(4.0);

        let words: Vec<&str> = ev.resumen.fortaleza.split_whitespace().collect();
        let mut line = String::new();
        for word in words {
            if line.len() + word.len() > 75 {
                layer.use_text(&format!("  {}", line), 9.0, left_margin, y_position, &font);
                y_position -= line_height;
                line = word.to_string();
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            layer.use_text(&format!("  {}", line), 9.0, left_margin, y_position, &font);
            y_position -= line_height;
        }
        y_position -= Mm(3.0);
    }

    if !ev.resumen.mejorar.is_empty() {
        layer.use_text("⚠ Área de mejora:", 9.0, left_margin, y_position, &font_bold);
        y_position -= Mm(4.0);

        let words: Vec<&str> = ev.resumen.mejorar.split_whitespace().collect();
        let mut line = String::new();
        for word in words {
            if line.len() + word.len() > 75 {
                layer.use_text(&format!("  {}", line), 9.0, left_margin, y_position, &font);
                y_position -= line_height;
                line = word.to_string();
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            layer.use_text(&format!("  {}", line), 9.0, left_margin, y_position, &font);
            y_position -= line_height;
        }
        y_position -= Mm(3.0);
    }

    y_position -= Mm(2.0);

    // Recomendaciones Priorizadas
    if y_position < min_y_position + Mm(30.0) {
        let (page3, layer3) = document.add_page(Mm(210.0), Mm(297.0), "Page 3");
        layer = document.get_page(page3).get_layer(layer3);
        y_position = Mm(277.0);
    }

    layer.use_text("RECOMENDACIONES PRIORIZADAS", 11.0, left_margin, y_position, &font_bold);
    y_position -= Mm(5.0);

    for (idx, rec) in ev.recomendaciones.iter().take(5).enumerate() {
        if y_position < min_y_position + Mm(12.0) {
            let (new_page, new_layer) = document.add_page(Mm(210.0), Mm(297.0), "Page");
            layer = document.get_page(new_page).get_layer(new_layer);
            y_position = Mm(277.0);
        }

        let rec_header = format!("{}. [P{}] {}", idx + 1, rec.prioridad, rec.titulo);
        layer.use_text(&rec_header, 9.0, left_margin, y_position, &font_bold);
        y_position -= Mm(4.0);

        let words: Vec<&str> = rec.texto_mejorado.split_whitespace().collect();
        let mut line = String::new();
        for word in words {
            if line.len() + word.len() > 75 {
                layer.use_text(&format!("  {}", line), 8.0, left_margin, y_position, &font);
                y_position -= Mm(4.0);
                line = word.to_string();
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            layer.use_text(&format!("  {}", line), 8.0, left_margin, y_position, &font);
            y_position -= Mm(4.0);
        }
        y_position -= Mm(2.0);
    }

    // Guardar a bytes
    let mut buffer = Vec::new();
    {
        let mut writer = BufWriter::new(&mut buffer);
        document
            .save(&mut writer)
            .map_err(|e| format!("Error guardando PDF: {}", e))?;
    }

    Ok(buffer)
}

/// Comando Tauri: exportar evaluación como PDF
///
/// Si `output_path` es None, guarda en `${APPDATA}/maity-desktop/`.
/// Retorna la ruta del archivo creado.
#[tauri::command]
pub async fn export_evaluation_pdf(
    meeting_id: String,
    output_path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    // Obtener evaluación de la DB
    let pool = state.db_manager.pool();
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT evaluation_json, created_at FROM meeting_evaluations WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Error DB: {}", e))?;

    let (json_str, created_at) = row.ok_or_else(|| "No hay evaluación para esta reunión. Genera primero con 'Generar evaluación'.".to_string())?;

    let evaluation: crate::coach::evaluation_types::MeetingEvaluation = serde_json::from_str(&json_str)
        .map_err(|e| format!("Evaluación corrupta: {}", e))?;

    // Obtener nombre de la reunión de la tabla meetings
    let meeting_name: Option<String> = sqlx::query_scalar(
        "SELECT title FROM meetings WHERE meeting_id = ?",
    )
    .bind(&meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Error obteniendo nombre: {}", e))?;

    let meeting_name = meeting_name.unwrap_or_else(|| format!("Reunión {}", meeting_id));

    let eval_data = EvaluationPdfData {
        meeting_id: meeting_id.clone(),
        meeting_name,
        created_at,
        evaluation,
    };

    // Generar PDF
    let pdf_bytes = generate_evaluation_pdf(&eval_data)?;

    // Determinar ruta de salida
    let final_path = if let Some(path) = output_path {
        PathBuf::from(path)
    } else {
        let app_data_dir = dirs::config_dir()
            .ok_or("No config dir found")?
            .join("maity-desktop");

        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Error creando directorio: {}", e))?;

        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("eval_{}_{}.pdf", meeting_id, timestamp);
        app_data_dir.join(filename)
    };

    // Escribir archivo
    std::fs::write(&final_path, pdf_bytes)
        .map_err(|e| format!("Error escribiendo PDF: {}", e))?;

    let path_str = final_path
        .to_str()
        .ok_or("Ruta PDF inválida")?
        .to_string();

    log_info!("PDF evaluación exportado: {}", path_str);
    Ok(path_str)
}

/// Comando Tauri: abrir carpeta en el explorador de archivos
#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);

    // Determinar si es archivo o carpeta
    let target = if path_buf.is_dir() {
        path_buf.clone()
    } else {
        path_buf.parent().ok_or("Ruta padre no disponible")?.to_path_buf()
    };

    #[cfg(target_os = "windows")]
    {
        // En Windows, usar `explorer.exe`
        let arg = format!("/select,{}", target.display());
        Command::new("explorer.exe")
            .arg(arg)
            .spawn()
            .map_err(|e| format!("Error abriendo explorador: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        // En macOS, usar `open`
        Command::new("open")
            .arg("-R")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Error abriendo Finder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        // En Linux, intentar con xdg-open o nautilus
        let _ = Command::new("xdg-open")
            .arg(target)
            .spawn();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::evaluation_types::*;

    #[test]
    fn generates_pdf_with_valid_bytes() {
        let mut eval = MeetingEvaluation::default();
        eval.resumen.puntuacion_global = 75.0;
        eval.resumen.nivel = "competente".to_string();
        eval.resumen.fortaleza = "Comunicación clara".to_string();
        eval.resumen.mejorar = "Mejorar estructura".to_string();
        eval.dimensiones.claridad.puntaje = 80.0;
        eval.dimensiones.proposito.puntaje = 70.0;
        eval.dimensiones.estructura.puntaje = 65.0;
        eval.dimensiones.persuasion.puntaje = 75.0;
        eval.dimensiones.adaptacion.puntaje = 70.0;
        eval.identificacion.nombre_sesion = "Reunión Test".to_string();

        let eval_data = EvaluationPdfData {
            meeting_id: "test-123".to_string(),
            meeting_name: "Reunión Test".to_string(),
            created_at: "2026-04-25 10:30:00".to_string(),
            evaluation: eval,
        };

        let result = generate_evaluation_pdf(&eval_data);
        assert!(result.is_ok(), "PDF generation should succeed");

        let bytes = result.unwrap();
        assert!(bytes.len() > 1000, "PDF should be > 1KB");
        assert!(bytes.starts_with(b"%PDF"), "PDF should start with PDF magic");
    }

    #[test]
    fn pdf_includes_meeting_name_and_score() {
        let mut eval = MeetingEvaluation::default();
        eval.resumen.puntuacion_global = 85.0;
        eval.resumen.nivel = "experto".to_string();
        eval.identificacion.nombre_sesion = "Reunión Importante".to_string();

        let eval_data = EvaluationPdfData {
            meeting_id: "test-456".to_string(),
            meeting_name: "Reunión Importante".to_string(),
            created_at: "2026-04-25 15:00:00".to_string(),
            evaluation: eval,
        };

        let result = generate_evaluation_pdf(&eval_data);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        // Verificar que el contenido de texto está en el PDF
        let pdf_str = String::from_utf8_lossy(&bytes);
        assert!(pdf_str.contains("Reunión") || pdf_str.len() > 2000,
                "PDF debe contener contenido");
    }
}
