//! Test runner de tips: lee scenarios con ground truth + transcripts ya
//! procesados (de dev_iterations), invoca coach_suggest sobre ventanas
//! deslizantes, calcula similarity vs expected_tips, persiste a tip_tests.

use crate::coach::commands::CoachSuggestion;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::path::PathBuf;
use tauri::Manager;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedTip {
    pub category: String,
    pub tip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthScenario {
    pub meeting_type: String,
    pub context_summary: String,
    pub expected_tips: Vec<ExpectedTip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthFile {
    pub version: String,
    pub scenarios: std::collections::HashMap<String, GroundTruthScenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipTestSummary {
    pub run_id: String,
    pub total_scenarios: usize,
    pub tips_generated: usize,
    pub avg_latency_ms: u64,
    pub duplicates: usize,
    pub avg_similarity: f32,
}

/// Tokeniza español: lowercase + strip puntuación + remueve stopwords cortas.
fn tokenize(s: &str) -> std::collections::HashSet<String> {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|w| w.len() >= 4) // skip short words
        .map(|w| w.to_string())
        .collect()
}

/// Jaccard similarity entre dos textos.
fn jaccard(a: &str, b: &str) -> f32 {
    let ta = tokenize(a);
    let tb = tokenize(b);
    if ta.is_empty() && tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

/// Mejor similarity de un tip generado contra cada expected (Jaccard fallback).
fn best_match_score(generated: &str, expected: &[ExpectedTip]) -> f32 {
    expected
        .iter()
        .map(|e| jaccard(generated, &e.tip))
        .fold(0.0f32, f32::max)
}

/// Mejor similarity vía embeddings cosine. Más fiel a significado semántico que Jaccard.
/// Falla silenciosamente y retorna None si nomic-embed-text no responde.
async fn best_match_score_semantic(
    generated: &str,
    expected_embeds: &[Vec<f32>],
    embed_model: &str,
) -> Option<f32> {
    let client = crate::coach::model_state::SHARED_CLIENT.clone();
    let gen_emb = crate::semantic_search::embedder::embed_text(&client, embed_model, generated, None)
        .await
        .ok()?;
    let mut best = 0.0f32;
    for exp_emb in expected_embeds {
        let sim = crate::semantic_search::cosine_similarity(&gen_emb, exp_emb);
        if sim > best {
            best = sim;
        }
    }
    Some(best)
}

/// Comando Tauri: corre tests de tips sobre todos los scenarios con ground truth.
/// Lee `test_data/tip_ground_truths.json` desde el binary location.
#[tauri::command]
pub async fn dev_run_tip_tests(
    app: tauri::AppHandle,
) -> Result<TipTestSummary, String> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState no disponible".to_string())?;
    let pool = state.db_manager.pool();

    // Resolve ground truth file
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let project_root = exe
        .parent()
        .and_then(|p| p.parent()) // target/release
        .and_then(|p| p.parent()) // target
        .map(|p| p.parent().unwrap_or(p).to_path_buf()) // workspace root
        .unwrap_or_else(|| PathBuf::from("."));

    let gt_paths = [
        project_root.join("test_data/tip_ground_truths.json"),
        PathBuf::from("D:/Proyectos de Kiro/Maity-desktop/test_data/tip_ground_truths.json"),
    ];

    let gt_content = gt_paths
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .ok_or_else(|| {
            format!(
                "No encontré tip_ground_truths.json en: {:?}",
                gt_paths.iter().collect::<Vec<_>>()
            )
        })?;

    let gt: GroundTruthFile = serde_json::from_str(&gt_content)
        .map_err(|e| format!("ground truth JSON inválido: {}", e))?;

    let run_id = format!(
        "tipsrun-{}",
        chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S")
    );
    let build_version = "v20".to_string();

    // v20: pre-computar embeddings de TODOS los expected_tips por scenario.
    // Usa nomic-embed-text (Ollama, 768d). Si falla, sigue con Jaccard fallback.
    let embed_model = "nomic-embed-text";
    let mut expected_embeds_per_scenario: std::collections::HashMap<String, Vec<Vec<f32>>> =
        std::collections::HashMap::new();
    let embed_client = crate::coach::model_state::SHARED_CLIENT.clone();
    let mut semantic_available = true;

    for (scenario_name, ground) in gt.scenarios.iter() {
        let mut embeds: Vec<Vec<f32>> = Vec::with_capacity(ground.expected_tips.len());
        for expected in &ground.expected_tips {
            match crate::semantic_search::embedder::embed_text(
                &embed_client, embed_model, &expected.tip, None,
            ).await {
                Ok(v) => embeds.push(v),
                Err(e) => {
                    log::warn!("[tip_tester] embed failed for expected tip ({}): {} — fallback Jaccard", scenario_name, e);
                    semantic_available = false;
                    break;
                }
            }
        }
        if !semantic_available {
            break;
        }
        expected_embeds_per_scenario.insert(scenario_name.clone(), embeds);
    }

    log::info!(
        "[tip_tester] STARTED run_id={} scenarios={} build_version={} semantic={}",
        run_id,
        gt.scenarios.len(),
        build_version,
        semantic_available
    );

    let mut total_tips = 0usize;
    let mut total_latency: u64 = 0;
    let mut duplicates = 0usize;
    let mut sim_sum = 0.0f32;

    for (scenario_name, ground) in gt.scenarios.iter() {
        // Buscar el último transcript hypothesis del scenario en dev_iterations
        let row = sqlx::query(
            "SELECT hypothesis_full FROM dev_iterations
             WHERE iteration_label = ? AND hypothesis_full IS NOT NULL
             ORDER BY id DESC LIMIT 1",
        )
        .bind(scenario_name)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

        let hypothesis: String = match row {
            Some(r) => r.get("hypothesis_full"),
            None => {
                log::warn!("[tip_tester] no transcript for scenario {}", scenario_name);
                continue;
            }
        };

        // 5 ventanas deslizantes (cada una últimos 600 chars del corte progresivo).
        // UTF-8 safe: trabajamos con char_indices para evitar romper multibytes ('á', 'ñ').
        let chars_count = hypothesis.chars().count();
        let chunk_chars = chars_count / 5;
        let mut prev_tips: Vec<String> = Vec::new();

        for chunk_idx in 1..=5 {
            let end_char = (chunk_idx * chunk_chars).min(chars_count);
            if end_char < 50 {
                continue;
            }
            let start_char = end_char.saturating_sub(600);
            let window: String = hypothesis
                .chars()
                .skip(start_char)
                .take(end_char - start_char)
                .collect();

            let role = "user".to_string();
            let language = "es-MX".to_string();
            let meeting_type = Some(ground.meeting_type.clone());

            // v20 #3: rotación de category hint + trigger signal según chunk_idx.
            // Esto fuerza al coach a explorar distintas dimensiones (objection,
            // discovery, closing, etc.) en lugar de caer siempre en el mismo
            // patrón "pregunta empática". Más fiel a uso real (donde trigger
            // detector sí pasa señales contextuales).
            let category_hints = match ground.meeting_type.as_str() {
                "sales" => vec!["discovery", "objection", "closing", "rapport", "negotiation"],
                "service" => vec!["empathy", "discovery", "ownership", "closing", "tone"],
                "team_meeting" => vec!["structure", "alignment", "data_request", "closing", "facilitation"],
                "coaching" => vec!["deep_question", "silence", "reformulate", "rapport", "introspective"],
                _ => vec!["rapport", "discovery", "listening", "pacing", "closing"],
            };
            let suggested_category = Some(category_hints[(chunk_idx - 1) % category_hints.len()].to_string());
            let trigger_signal = Some(if chunk_idx % 2 == 0 {
                "last_speaker_interlocutor".to_string()
            } else {
                "last_speaker_user".to_string()
            });

            let t0 = std::time::Instant::now();
            let result: Result<CoachSuggestion, String> = crate::coach::commands::coach_suggest(
                app.clone(),
                window.clone(),
                role,
                language,
                None,
                meeting_type,
                Some(chunk_idx as u32),
                Some(prev_tips.clone()),
                suggested_category,
                trigger_signal,
            )
            .await;
            let latency_ms = t0.elapsed().as_millis() as u64;

            match result {
                Ok(suggestion) => {
                    let generated = suggestion.tip.clone();
                    let category = suggestion.category.clone();
                    let confidence = suggestion.confidence;

                    // Dedup check vs prev_tips
                    let is_dup = prev_tips
                        .iter()
                        .any(|p| jaccard(p, &generated) > 0.55);

                    // v20: prefer semantic similarity (cosine on embeddings),
                    // fallback Jaccard si nomic-embed-text no estaba disponible.
                    let sim = if semantic_available {
                        if let Some(exp_embs) = expected_embeds_per_scenario.get(scenario_name) {
                            best_match_score_semantic(&generated, exp_embs, embed_model)
                                .await
                                .unwrap_or_else(|| best_match_score(&generated, &ground.expected_tips))
                        } else {
                            best_match_score(&generated, &ground.expected_tips)
                        }
                    } else {
                        best_match_score(&generated, &ground.expected_tips)
                    };
                    let novelty = if prev_tips.is_empty() {
                        1.0
                    } else {
                        1.0 - prev_tips
                            .iter()
                            .map(|p| jaccard(p, &generated))
                            .fold(0.0f32, f32::max)
                    };

                    let expected_json = serde_json::to_string(&ground.expected_tips).unwrap_or_default();

                    sqlx::query(
                        "INSERT INTO tip_tests (
                            scenario, test_run_id, build_version, context_window,
                            meeting_type, expected_tips, generated_tip, generated_category,
                            generated_confidence, latency_ms, similarity_score, is_duplicate,
                            novelty_score, notes
                         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(scenario_name)
                    .bind(&run_id)
                    .bind(&build_version)
                    .bind(window.chars().take(120).collect::<String>())
                    .bind(&ground.meeting_type)
                    .bind(expected_json)
                    .bind(&generated)
                    .bind(&category)
                    .bind(confidence as f64)
                    .bind(latency_ms as i64)
                    .bind(sim as f64)
                    .bind(if is_dup { 1 } else { 0 })
                    .bind(novelty as f64)
                    .bind(format!("chunk {}/5 minute {}", chunk_idx, chunk_idx))
                    .execute(pool)
                    .await
                    .map_err(|e| format!("insert tip_test: {}", e))?;

                    total_tips += 1;
                    total_latency += latency_ms;
                    if is_dup {
                        duplicates += 1;
                    }
                    sim_sum += sim;

                    prev_tips.push(generated);
                    if prev_tips.len() > 5 {
                        prev_tips.remove(0);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "[tip_tester] coach_suggest failed for {} chunk {}: {}",
                        scenario_name, chunk_idx, e
                    );
                    sqlx::query(
                        "INSERT INTO tip_tests (
                            scenario, test_run_id, build_version,
                            meeting_type, expected_tips, generated_tip, latency_ms, notes
                         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(scenario_name)
                    .bind(&run_id)
                    .bind(&build_version)
                    .bind(&ground.meeting_type)
                    .bind(serde_json::to_string(&ground.expected_tips).unwrap_or_default())
                    .bind(format!("[ERROR] {}", e))
                    .bind(latency_ms as i64)
                    .bind("coach_suggest failed")
                    .execute(pool)
                    .await
                    .ok();
                }
            }

            let _ = Uuid::new_v4();
        }
    }

    let avg_latency = if total_tips > 0 {
        total_latency / total_tips as u64
    } else {
        0
    };
    let avg_similarity = if total_tips > 0 {
        sim_sum / total_tips as f32
    } else {
        0.0
    };

    Ok(TipTestSummary {
        run_id,
        total_scenarios: gt.scenarios.len(),
        tips_generated: total_tips,
        avg_latency_ms: avg_latency,
        duplicates,
        avg_similarity,
    })
}
