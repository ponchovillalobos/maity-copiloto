//! Comando dev para importar un archivo de audio (mp3/wav/m4a) y ejecutar
//! el pipeline completo: decodificación → transcripción Parakeet → tips +
//! evaluación. Útil para iterar sobre la calidad sin grabar reuniones reales.
//!
//! La ruta solo se expone en `/dev` del frontend (no aparece en navegación
//! principal). En producción se puede deshabilitar via condicional render.

use crate::api::api::TranscriptSegment;
use crate::audio::ffmpeg::find_ffmpeg_path;
use crate::database::repositories::transcript::TranscriptsRepository;
use crate::parakeet_engine::commands::PARAKEET_ENGINE;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use uuid::Uuid;

/// Sample rate target para Parakeet (16 kHz mono, f32).
const TARGET_SAMPLE_RATE: u32 = 16000;
/// Tamaño de chunk de transcripción (30 s = 480_000 muestras a 16 kHz).
const CHUNK_SAMPLES: usize = (TARGET_SAMPLE_RATE as usize) * 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevImportProgress {
    pub stage: String,
    pub current_chunk: usize,
    pub total_chunks: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevImportResult {
    pub meeting_id: String,
    pub transcript_segments: usize,
    pub total_duration_seconds: f64,
}

/// Decodifica un archivo de audio a `Vec<f32>` mono 16 kHz usando ffmpeg.
fn decode_to_pcm_f32(input_path: &str) -> Result<Vec<f32>, String> {
    let ffmpeg = find_ffmpeg_path()
        .ok_or_else(|| "FFmpeg no encontrado. Reinstala la app.".to_string())?;

    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-i",
        input_path,
        "-f",
        "f32le",
        "-acodec",
        "pcm_f32le",
        "-ac",
        "1",
        "-ar",
        &TARGET_SAMPLE_RATE.to_string(),
        "pipe:1",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo lanzar ffmpeg: {}", e))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "ffmpeg stdout no disponible".to_string())?;

    let mut buf = Vec::with_capacity(1024 * 1024);
    stdout
        .read_to_end(&mut buf)
        .map_err(|e| format!("Error leyendo audio decodificado: {}", e))?;

    let status = child
        .wait()
        .map_err(|e| format!("ffmpeg wait error: {}", e))?;
    if !status.success() {
        return Err(format!(
            "ffmpeg falló decodificando '{}' (exit {:?})",
            input_path,
            status.code()
        ));
    }

    if buf.len() % 4 != 0 {
        return Err("Buffer de audio decodificado no múltiplo de 4 bytes (no es f32)".to_string());
    }

    let samples: Vec<f32> = buf
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Ok(samples)
}

/// Transcribe un único chunk usando el engine Parakeet ya inicializado.
async fn transcribe_chunk(samples: Vec<f32>) -> Result<String, String> {
    let engine = {
        let guard = PARAKEET_ENGINE
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        guard.as_ref().cloned()
    };
    let engine = engine.ok_or_else(|| {
        "Parakeet engine no inicializado. Asegúrate de haber completado el onboarding."
            .to_string()
    })?;
    engine
        .transcribe_audio(samples)
        .await
        .map_err(|e| format!("Parakeet error: {}", e))
}

/// Importa un archivo de audio y lo procesa end-to-end.
/// Retorna `meeting_id` para que el frontend pueda navegar a la reunión.
#[tauri::command]
pub async fn dev_import_audio_file<R: Runtime>(
    app: AppHandle<R>,
    file_path: String,
    meeting_name: Option<String>,
) -> Result<DevImportResult, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("Archivo no encontrado: {}", file_path));
    }

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "decoding".into(),
            current_chunk: 0,
            total_chunks: 0,
            message: "Decodificando audio…".into(),
        },
    );

    let samples = decode_to_pcm_f32(&file_path)?;
    let total_duration = samples.len() as f64 / TARGET_SAMPLE_RATE as f64;

    if samples.is_empty() {
        return Err("Audio decodificado vacío".to_string());
    }

    let chunks: Vec<&[f32]> = samples.chunks(CHUNK_SAMPLES).collect();
    let total_chunks = chunks.len();

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "transcribing".into(),
            current_chunk: 0,
            total_chunks,
            message: format!("Transcribiendo {} chunks…", total_chunks),
        },
    );

    let mut segments: Vec<TranscriptSegment> = Vec::with_capacity(total_chunks);

    for (idx, chunk) in chunks.iter().enumerate() {
        let chunk_vec = chunk.to_vec();
        let start_time = (idx * CHUNK_SAMPLES) as f64 / TARGET_SAMPLE_RATE as f64;
        let end_time = ((idx + 1) * CHUNK_SAMPLES).min(samples.len()) as f64
            / TARGET_SAMPLE_RATE as f64;

        let text = match transcribe_chunk(chunk_vec).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("[dev_import] Chunk {} fallo transcripción: {}", idx, e);
                String::new()
            }
        };

        if !text.trim().is_empty() {
            segments.push(TranscriptSegment {
                id: format!("transcript-{}", Uuid::new_v4()),
                text,
                timestamp: Utc::now().to_rfc3339(),
                audio_start_time: Some(start_time),
                audio_end_time: Some(end_time),
                duration: Some(end_time - start_time),
                source_type: Some("interlocutor".into()),
            });
        }

        let _ = app.emit(
            "dev-import-progress",
            DevImportProgress {
                stage: "transcribing".into(),
                current_chunk: idx + 1,
                total_chunks,
                message: format!("Chunk {}/{} listo", idx + 1, total_chunks),
            },
        );
    }

    if segments.is_empty() {
        return Err("Ningún chunk produjo transcripción. ¿Audio sin voz o muy bajo?".to_string());
    }

    let title = meeting_name
        .unwrap_or_else(|| format!("Test Audio {}", Utc::now().format("%Y-%m-%d %H:%M")));

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let meeting_id = TranscriptsRepository::save_transcript(pool, &title, &segments, None)
        .await
        .map_err(|e| format!("DB error guardando reunión: {}", e))?;

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "evaluating".into(),
            current_chunk: total_chunks,
            total_chunks,
            message: "Generando evaluación post-meeting…".into(),
        },
    );

    let full_transcript = segments
        .iter()
        .map(|s| format!("[{}] {}", s.source_type.as_deref().unwrap_or("?"), s.text))
        .collect::<Vec<_>>()
        .join("\n");

    if let Err(e) = crate::coach::evaluator::coach_evaluate_post_meeting(
        app.clone(),
        meeting_id.clone(),
        full_transcript,
        None,
        None,
    )
    .await
    {
        log::warn!("[dev_import] Evaluación falló: {}", e);
    }

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "done".into(),
            current_chunk: total_chunks,
            total_chunks,
            message: "Listo".into(),
        },
    );

    Ok(DevImportResult {
        meeting_id,
        transcript_segments: segments.len(),
        total_duration_seconds: total_duration,
    })
}
