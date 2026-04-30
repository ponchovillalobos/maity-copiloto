//! Comando dev para importar un archivo de audio (mp3/wav/m4a) y ejecutar
//! el pipeline completo: decodificación → transcripción Parakeet → tips +
//! evaluación. Útil para iterar sobre la calidad sin grabar reuniones reales.
//!
//! La ruta solo se expone en `/dev` del frontend (no aparece en navegación
//! principal). En producción se puede deshabilitar via condicional render.
//!
//! ## Formato de audio recomendado
//!
//! - **Estéreo** (recomendado): canal **L = micrófono (user)** + canal
//!   **R = sistema/interlocutor**. Maity detecta automáticamente y separa
//!   speakers correctamente. Coincide con el formato de las grabaciones
//!   reales (`recording_saver.rs` ya guarda L=mic R=sistema).
//! - **Mono** (fallback): un solo canal. Por defecto se etiqueta como
//!   `interlocutor`. Útil cuando solo grabaste a la otra persona.
//! - **Dos archivos separados**: usar el comando dos veces o mezclar a
//!   estéreo en Audacity primero (Tracks → Mix → Mix Stereo Down).

use crate::api::api::TranscriptSegment;
use crate::audio::ffmpeg::find_ffmpeg_path;
use crate::database::repositories::transcript::TranscriptsRepository;
use crate::observability::iteration_log::{insert_iteration, NewIterationRecord};
use crate::observability::timing::Timer;
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
    /// `"stereo"` si se detectaron 2 canales (L=mic, R=sistema), `"mono"` si 1.
    pub channel_layout: String,
    /// WER global (0.0 = perfecto, 1.0 = todo mal). Solo presente si se pasó ground_truth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wer_global: Option<crate::coach::wer::WerResult>,
    /// WER del canal user (mic) — solo si se pasó ground_truth_user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wer_user: Option<crate::coach::wer::WerResult>,
    /// WER del canal interlocutor (sistema) — solo si se pasó ground_truth_interlocutor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wer_interlocutor: Option<crate::coach::wer::WerResult>,
    /// Texto transcrito por Maity (concatenado, sin labels) — útil para ver el output crudo.
    pub maity_transcript_full: String,
}

/// Cuenta cuántas de las 15 secciones top-level del MeetingEvaluation tienen
/// contenido no-default. Útil para tracking de calidad de la evaluación.
pub fn count_sections_filled(eval: &crate::coach::evaluation_types::MeetingEvaluation) -> u32 {
    let mut count = 0u32;
    if !eval.identificacion.nombre_sesion.is_empty() { count += 1 }
    if eval.historico.sesion_anterior_id.is_some()
        || !eval.historico.mejoras_detectadas.is_empty()
        || !eval.historico.regresiones_detectadas.is_empty() { count += 1 }
    if !eval.contexto.relacion.is_empty() { count += 1 }
    if eval.meta.duracion_minutos > 0 || !eval.meta.tipo.is_empty() { count += 1 }
    if eval.resumen.puntuacion_global > 0.0 { count += 1 }
    if eval.radiografia.muletillas_total > 0 || !eval.radiografia.preguntas.is_empty() { count += 1 }
    if !eval.insights.is_empty() { count += 1 }
    if !eval.patron.actual.is_empty() { count += 1 }
    if !eval.timeline.segmentos.is_empty() || !eval.timeline.momentos_clave.is_empty() { count += 1 }
    if eval.dimensiones.claridad.puntaje > 0.0 || eval.dimensiones.estructura.puntaje > 0.0 { count += 1 }
    if !eval.por_hablante.is_empty() { count += 1 }
    if !eval.empatia.is_empty() { count += 1 }
    if eval.calidad_global.puntaje > 0.0 { count += 1 }
    if !eval.recomendaciones.is_empty() { count += 1 }
    if !eval.visualizaciones.gauge.label.is_empty() || !eval.visualizaciones.radar_calidad.labels.is_empty() { count += 1 }
    count
}

/// Detecta si el archivo es estéreo. Lee `ffmpeg -i` y busca "stereo" en stderr.
fn detect_is_stereo(input_path: &str) -> bool {
    let Some(ffmpeg) = find_ffmpeg_path() else {
        return false;
    };
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-i", input_path]).stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let Ok(output) = cmd.output() else { return false };
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("stereo")
}

/// Decodifica audio a PCM f32 16 kHz. Si `keep_stereo`=true, retorna interleaved
/// L/R (2 canales). Si false, mezcla a mono.
fn decode_to_pcm_f32(input_path: &str, keep_stereo: bool) -> Result<Vec<f32>, String> {
    let ffmpeg = find_ffmpeg_path()
        .ok_or_else(|| "FFmpeg no encontrado. Reinstala la app.".to_string())?;

    let channels = if keep_stereo { "2" } else { "1" };
    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-i",
        input_path,
        "-f",
        "f32le",
        "-acodec",
        "pcm_f32le",
        "-ac",
        channels,
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

/// Separa audio interleaved L/R en dos buffers mono.
fn deinterleave_stereo(interleaved: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut left = Vec::with_capacity(interleaved.len() / 2);
    let mut right = Vec::with_capacity(interleaved.len() / 2);
    let mut iter = interleaved.iter();
    while let (Some(&l), Some(&r)) = (iter.next(), iter.next()) {
        left.push(l);
        right.push(r);
    }
    (left, right)
}

/// Calcula RMS (energía) de un buffer. Útil para skip chunks silenciosos.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|x| x * x).sum();
    (sum / samples.len() as f32).sqrt()
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

/// Procesa un canal mono completo, devuelve segments con `source_type` dado.
async fn transcribe_channel(
    app: &AppHandle<impl Runtime>,
    samples: &[f32],
    source_type: &str,
    total_chunks_global: usize,
    base_chunk_idx: usize,
) -> Vec<TranscriptSegment> {
    let chunks: Vec<&[f32]> = samples.chunks(CHUNK_SAMPLES).collect();
    let mut out = Vec::with_capacity(chunks.len());
    // Threshold de silencio — chunks con RMS < 0.005 no se transcriben (skip).
    const SILENCE_RMS: f32 = 0.005;

    for (idx, chunk) in chunks.iter().enumerate() {
        let chunk_rms = rms(chunk);
        if chunk_rms < SILENCE_RMS {
            log::debug!("[dev_import] Chunk {} canal {} silencioso (RMS={:.4}), skip", idx, source_type, chunk_rms);
        } else {
            let chunk_vec = chunk.to_vec();
            let start_time = (idx * CHUNK_SAMPLES) as f64 / TARGET_SAMPLE_RATE as f64;
            let end_time = ((idx + 1) * CHUNK_SAMPLES).min(samples.len()) as f64
                / TARGET_SAMPLE_RATE as f64;

            let text = match transcribe_chunk(chunk_vec).await {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("[dev_import] Chunk {} canal {} fallo: {}", idx, source_type, e);
                    String::new()
                }
            };

            if !text.trim().is_empty() {
                out.push(TranscriptSegment {
                    id: format!("transcript-{}", Uuid::new_v4()),
                    text,
                    timestamp: Utc::now().to_rfc3339(),
                    audio_start_time: Some(start_time),
                    audio_end_time: Some(end_time),
                    duration: Some(end_time - start_time),
                    source_type: Some(source_type.to_string()),
                });
            }
        }

        let _ = app.emit(
            "dev-import-progress",
            DevImportProgress {
                stage: "transcribing".into(),
                current_chunk: base_chunk_idx + idx + 1,
                total_chunks: total_chunks_global,
                message: format!("Canal {} chunk {}/{}", source_type, idx + 1, chunks.len()),
            },
        );
    }
    out
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

    let pipeline_timer_single = Timer::start();

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "decoding".into(),
            current_chunk: 0,
            total_chunks: 0,
            message: "Decodificando audio…".into(),
        },
    );

    let is_stereo = detect_is_stereo(&file_path);
    let layout_label = if is_stereo { "stereo" } else { "mono" };
    log::info!("[dev_import] Audio detectado como {}", layout_label);

    let decode_timer_single = Timer::start();
    let samples = decode_to_pcm_f32(&file_path, is_stereo)?;
    let decode_ms_single = decode_timer_single.elapsed_ms() as i64;
    let frame_count = if is_stereo { samples.len() / 2 } else { samples.len() };
    let total_duration = frame_count as f64 / TARGET_SAMPLE_RATE as f64;

    if samples.is_empty() {
        return Err("Audio decodificado vacío".to_string());
    }

    let mut all_segments: Vec<TranscriptSegment> = Vec::new();

    if is_stereo {
        let (left, right) = deinterleave_stereo(&samples);
        let chunks_per_channel = (left.len() + CHUNK_SAMPLES - 1) / CHUNK_SAMPLES;
        let total_chunks_global = chunks_per_channel * 2;

        let _ = app.emit(
            "dev-import-progress",
            DevImportProgress {
                stage: "transcribing".into(),
                current_chunk: 0,
                total_chunks: total_chunks_global,
                message: format!(
                    "Estéreo detectado: L=user, R=interlocutor ({} chunks/canal)",
                    chunks_per_channel
                ),
            },
        );

        let user_segments = transcribe_channel(&app, &left, "user", total_chunks_global, 0).await;
        let inter_segments =
            transcribe_channel(&app, &right, "interlocutor", total_chunks_global, chunks_per_channel)
                .await;

        all_segments.extend(user_segments);
        all_segments.extend(inter_segments);
        all_segments.sort_by(|a, b| {
            a.audio_start_time
                .partial_cmp(&b.audio_start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        let chunks_count = (samples.len() + CHUNK_SAMPLES - 1) / CHUNK_SAMPLES;
        let _ = app.emit(
            "dev-import-progress",
            DevImportProgress {
                stage: "transcribing".into(),
                current_chunk: 0,
                total_chunks: chunks_count,
                message: format!(
                    "Mono: marcando todo como interlocutor ({} chunks)",
                    chunks_count
                ),
            },
        );
        let segs = transcribe_channel(&app, &samples, "interlocutor", chunks_count, 0).await;
        all_segments.extend(segs);
    }

    if all_segments.is_empty() {
        return Err("Ningún chunk produjo transcripción. ¿Audio sin voz o muy bajo?".to_string());
    }

    let title = meeting_name
        .unwrap_or_else(|| format!("Test Audio {}", Utc::now().format("%Y-%m-%d %H:%M")));

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    let meeting_id = TranscriptsRepository::save_transcript(pool, &title, &all_segments, None)
        .await
        .map_err(|e| format!("DB error guardando reunión: {}", e))?;

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "evaluating".into(),
            current_chunk: 0,
            total_chunks: 1,
            message: "Generando evaluación post-meeting…".into(),
        },
    );

    let full_transcript = all_segments
        .iter()
        .map(|s| format!("[{}] {}", s.source_type.as_deref().unwrap_or("?"), s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let eval_timer_single = Timer::start();
    let eval_result_single = crate::coach::evaluator::coach_evaluate_post_meeting(
        app.clone(),
        meeting_id.clone(),
        full_transcript,
        None,
        None,
    )
    .await;
    let evaluation_ms_single = eval_timer_single.elapsed_ms() as i64;

    let (eval_score_single, eval_sections_single) = match &eval_result_single {
        Ok(r) => (
            Some(r.evaluation.resumen.puntuacion_global),
            Some(count_sections_filled(&r.evaluation) as i64),
        ),
        Err(e) => {
            log::warn!("[dev_import] Evaluación falló: {}", e);
            (None, None)
        }
    };

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "done".into(),
            current_chunk: 1,
            total_chunks: 1,
            message: "Listo".into(),
        },
    );

    let maity_full = all_segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let total_pipeline_ms_single = pipeline_timer_single.elapsed_ms() as i64;
    let _ = insert_iteration(
        pool,
        &NewIterationRecord {
            meeting_id: meeting_id.clone(),
            iteration_label: Some(title.clone()),
            audio_user_path: Some(file_path.clone()),
            audio_interlocutor_path: None,
            channel_layout: layout_label.to_string(),
            total_duration_seconds: total_duration,
            decode_ms: Some(decode_ms_single),
            transcribe_user_ms: None,
            transcribe_interlocutor_ms: None,
            evaluation_ms: Some(evaluation_ms_single),
            total_pipeline_ms: Some(total_pipeline_ms_single),
            wer_global: None,
            wer_user: None,
            wer_interlocutor: None,
            hypothesis_full: Some(maity_full.clone()),
            reference_user: None,
            reference_interlocutor: None,
            evaluation_score: eval_score_single,
            evaluation_sections_filled: eval_sections_single,
            prompt_version: "v3-lite + eval-v4".into(),
            coach_model: "qwen3:0.6b".into(),
            evaluation_model: "qwen3:1.7b".into(),
            cpu_avg_pct: None,
            ram_peak_mb: None,
            notes: None,
        },
    )
    .await
    .map_err(|e| log::warn!("[dev_import] insert_iteration falló: {}", e))
    .ok();

    Ok(DevImportResult {
        meeting_id,
        transcript_segments: all_segments.len(),
        total_duration_seconds: total_duration,
        channel_layout: layout_label.to_string(),
        wer_global: None,
        wer_user: None,
        wer_interlocutor: None,
        maity_transcript_full: maity_full,
    })
}

// ============================================================================
// QA: 2 audios separados + ground truth → WER por canal y global
// ============================================================================

/// Importa DOS archivos de audio (uno por speaker) + ground truth opcional.
/// Calcula WER (Word Error Rate) por canal y global. Útil para iteración 100×.
///
/// - `user_audio_path`: archivo del micrófono (tu voz)
/// - `interlocutor_audio_path`: archivo del sistema/cliente
/// - `ground_truth_user`: texto exacto que dijiste (opcional)
/// - `ground_truth_interlocutor`: texto exacto del cliente (opcional)
/// - `meeting_name`: nombre custom de la reunión generada
#[tauri::command]
pub async fn dev_import_two_audios<R: Runtime>(
    app: AppHandle<R>,
    user_audio_path: String,
    interlocutor_audio_path: String,
    ground_truth_user: Option<String>,
    ground_truth_interlocutor: Option<String>,
    meeting_name: Option<String>,
) -> Result<DevImportResult, String> {
    if !PathBuf::from(&user_audio_path).exists() {
        return Err(format!("Archivo user no encontrado: {}", user_audio_path));
    }
    if !PathBuf::from(&interlocutor_audio_path).exists() {
        return Err(format!(
            "Archivo interlocutor no encontrado: {}",
            interlocutor_audio_path
        ));
    }

    let pipeline_timer = Timer::start();

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "decoding".into(),
            current_chunk: 0,
            total_chunks: 0,
            message: "Decodificando ambos canales…".into(),
        },
    );

    let decode_timer = Timer::start();
    let user_samples = decode_to_pcm_f32(&user_audio_path, false)?;
    let inter_samples = decode_to_pcm_f32(&interlocutor_audio_path, false)?;
    let decode_ms = decode_timer.elapsed_ms() as i64;

    if user_samples.is_empty() && inter_samples.is_empty() {
        return Err("Ambos audios vacíos".to_string());
    }

    let max_frames = user_samples.len().max(inter_samples.len());
    let total_duration = max_frames as f64 / TARGET_SAMPLE_RATE as f64;

    let user_chunks_count = (user_samples.len() + CHUNK_SAMPLES - 1) / CHUNK_SAMPLES;
    let inter_chunks_count = (inter_samples.len() + CHUNK_SAMPLES - 1) / CHUNK_SAMPLES;
    let total_chunks_global = user_chunks_count + inter_chunks_count;

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "transcribing".into(),
            current_chunk: 0,
            total_chunks: total_chunks_global,
            message: format!(
                "2 audios: user={}c, interlocutor={}c",
                user_chunks_count, inter_chunks_count
            ),
        },
    );

    let user_timer = Timer::start();
    let user_segments =
        transcribe_channel(&app, &user_samples, "user", total_chunks_global, 0).await;
    let transcribe_user_ms = user_timer.elapsed_ms() as i64;

    let inter_timer = Timer::start();
    let inter_segments = transcribe_channel(
        &app,
        &inter_samples,
        "interlocutor",
        total_chunks_global,
        user_chunks_count,
    )
    .await;
    let transcribe_interlocutor_ms = inter_timer.elapsed_ms() as i64;

    // Capturar texto crudo por canal antes de fusionar (para WER por canal).
    let user_text: String = user_segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let inter_text: String = inter_segments
        .iter()
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let mut all_segments: Vec<TranscriptSegment> = Vec::new();
    all_segments.extend(user_segments);
    all_segments.extend(inter_segments);
    all_segments.sort_by(|a, b| {
        a.audio_start_time
            .partial_cmp(&b.audio_start_time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if all_segments.is_empty() {
        return Err("Ningún chunk produjo transcripción".to_string());
    }

    let title = meeting_name
        .unwrap_or_else(|| format!("QA {}", Utc::now().format("%Y-%m-%d %H:%M")));

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();
    let meeting_id = TranscriptsRepository::save_transcript(pool, &title, &all_segments, None)
        .await
        .map_err(|e| format!("DB error guardando reunión: {}", e))?;

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "evaluating".into(),
            current_chunk: 0,
            total_chunks: 1,
            message: "Generando evaluación post-meeting…".into(),
        },
    );

    let full_transcript_for_eval = all_segments
        .iter()
        .map(|s| format!("[{}] {}", s.source_type.as_deref().unwrap_or("?"), s.text))
        .collect::<Vec<_>>()
        .join("\n");

    let eval_timer = Timer::start();
    let eval_result = crate::coach::evaluator::coach_evaluate_post_meeting(
        app.clone(),
        meeting_id.clone(),
        full_transcript_for_eval,
        None,
        None,
    )
    .await;
    let evaluation_ms = eval_timer.elapsed_ms() as i64;

    let (evaluation_score, evaluation_sections_filled) = match &eval_result {
        Ok(r) => (
            Some(r.evaluation.resumen.puntuacion_global),
            Some(count_sections_filled(&r.evaluation) as i64),
        ),
        Err(e) => {
            log::warn!("[dev_import_two] Evaluación falló: {}", e);
            (None, None)
        }
    };

    use crate::coach::wer::compute_wer;
    let wer_user = ground_truth_user
        .as_deref()
        .map(|gt| compute_wer(gt, &user_text));
    let wer_inter = ground_truth_interlocutor
        .as_deref()
        .map(|gt| compute_wer(gt, &inter_text));

    let wer_global = match (
        ground_truth_user.as_deref(),
        ground_truth_interlocutor.as_deref(),
    ) {
        (Some(gu), Some(gi)) => {
            let combined_ref = format!("{} {}", gu, gi);
            let combined_hyp = format!("{} {}", user_text, inter_text);
            Some(compute_wer(&combined_ref, &combined_hyp))
        }
        _ => None,
    };

    let total_pipeline_ms = pipeline_timer.elapsed_ms() as i64;

    let _ = insert_iteration(
        pool,
        &NewIterationRecord {
            meeting_id: meeting_id.clone(),
            iteration_label: Some(title.clone()),
            audio_user_path: Some(user_audio_path.clone()),
            audio_interlocutor_path: Some(interlocutor_audio_path.clone()),
            channel_layout: "two-files".into(),
            total_duration_seconds: total_duration,
            decode_ms: Some(decode_ms),
            transcribe_user_ms: Some(transcribe_user_ms),
            transcribe_interlocutor_ms: Some(transcribe_interlocutor_ms),
            evaluation_ms: Some(evaluation_ms),
            total_pipeline_ms: Some(total_pipeline_ms),
            wer_global: wer_global.as_ref().map(|w| w.wer),
            wer_user: wer_user.as_ref().map(|w| w.wer),
            wer_interlocutor: wer_inter.as_ref().map(|w| w.wer),
            hypothesis_full: Some(format!("[user] {}\n[interlocutor] {}", user_text, inter_text)),
            reference_user: ground_truth_user.clone(),
            reference_interlocutor: ground_truth_interlocutor.clone(),
            evaluation_score,
            evaluation_sections_filled,
            prompt_version: "v3-lite + eval-v4".into(),
            coach_model: "qwen3:0.6b".into(),
            evaluation_model: "qwen3:1.7b".into(),
            cpu_avg_pct: None,
            ram_peak_mb: None,
            notes: None,
        },
    )
    .await
    .map_err(|e| log::warn!("[dev_import_two] insert_iteration falló: {}", e))
    .ok();

    let _ = app.emit(
        "dev-import-progress",
        DevImportProgress {
            stage: "done".into(),
            current_chunk: 1,
            total_chunks: 1,
            message: match &wer_global {
                Some(w) => format!("WER global: {:.1}%", w.wer * 100.0),
                None => "Listo".into(),
            },
        },
    );

    let maity_full = format!("[user] {}\n[interlocutor] {}", user_text, inter_text);

    Ok(DevImportResult {
        meeting_id,
        transcript_segments: all_segments.len(),
        total_duration_seconds: total_duration,
        channel_layout: "two-files".to_string(),
        wer_global,
        wer_user,
        wer_interlocutor: wer_inter,
        maity_transcript_full: maity_full,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deinterleave_stereo_basic() {
        let interleaved = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (l, r) = deinterleave_stereo(&interleaved);
        assert_eq!(l, vec![1.0, 3.0, 5.0]);
        assert_eq!(r, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_deinterleave_stereo_empty() {
        let (l, r) = deinterleave_stereo(&[]);
        assert!(l.is_empty());
        assert!(r.is_empty());
    }

    #[test]
    fn test_deinterleave_stereo_odd_length() {
        let interleaved = vec![1.0, 2.0, 3.0];
        let (l, r) = deinterleave_stereo(&interleaved);
        assert_eq!(l, vec![1.0]);
        assert_eq!(r, vec![2.0]);
    }

    #[test]
    fn test_rms_silence() {
        let samples = vec![0.0; 1000];
        assert_eq!(rms(&samples), 0.0);
    }

    #[test]
    fn test_rms_constant() {
        let samples = vec![0.5; 100];
        assert!((rms(&samples) - 0.5).abs() < 1e-6);
    }
}
