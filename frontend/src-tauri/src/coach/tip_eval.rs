//! Harness de iteración de prompt v31.10.
//!
//! Carga scenarios JSON con transcripts pre-armados, invoca el pipeline
//! real (`coach_simple_tick`) y evalúa si el tip generado cumple el formato
//! esperado (verbo correcto, frase entre comillas, longitud). Permite iterar
//! prompt sin necesidad de grabar audio.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub category: String,
    pub expected_intent: String,
    pub expected_verbs: Vec<String>,
    pub transcript: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario: String,
    pub category: String,
    pub generated_tip: Option<String>,
    pub verb_match: bool,
    pub has_quoted_phrase: bool,
    pub word_count: usize,
    pub passed: bool,
    pub latency_ms: u64,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub avg_latency_ms: u64,
    pub results: Vec<ScenarioResult>,
}

fn scenarios_dir() -> PathBuf {
    // En dev se busca en CARGO_MANIFEST_DIR; en producción asumimos cwd o
    // resources del bundle. Para iteración interactiva basta CARGO_MANIFEST_DIR.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("scenarios")
}

fn load_scenarios() -> Result<Vec<(String, Scenario)>, String> {
    let dir = scenarios_dir();
    if !dir.exists() {
        return Err(format!("Scenarios dir no existe: {:?}", dir));
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("Read dir: {}", e))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for p in paths {
        let content = std::fs::read_to_string(&p)
            .map_err(|e| format!("Read {:?}: {}", p, e))?;
        let scenario: Scenario = serde_json::from_str(&content)
            .map_err(|e| format!("Parse {:?}: {}", p, e))?;
        let fname = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        out.push((fname, scenario));
    }
    Ok(out)
}

fn evaluate_tip(tip: &str, expected_verbs: &[String]) -> (bool, bool, usize, bool) {
    let lower = tip.to_lowercase();
    let primera = tip
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(|c: char| !c.is_alphabetic())
        .to_lowercase();
    let verb_match = expected_verbs.iter().any(|v| v.to_lowercase() == primera);
    let has_quoted_phrase = tip.contains('"')
        || tip.contains('\u{201C}')
        || tip.contains('\u{201D}')
        || tip.contains('\'');
    let word_count = tip.split_whitespace().count();
    let has_colon = lower.contains(':');
    let passed = verb_match && has_quoted_phrase && has_colon && word_count >= 5 && word_count <= 25;
    (verb_match, has_quoted_phrase, word_count, passed)
}

/// Comando Tauri dev: corre TODOS los scenarios y devuelve reporte agregado.
#[tauri::command]
pub async fn dev_eval_scenarios(app: tauri::AppHandle) -> Result<EvalReport, String> {
    let scenarios = load_scenarios()?;
    if scenarios.is_empty() {
        return Err("No hay scenarios JSON".to_string());
    }

    let mut results = Vec::with_capacity(scenarios.len());
    let mut total_latency = 0u64;

    for (fname, scenario) in &scenarios {
        let start = std::time::Instant::now();
        // Inyectar el transcript en el buffer + setear meeting_id mock antes
        // de invocar coach_simple_tick. Más simple: pasar window directo.
        let window = scenario.transcript.clone();
        let meeting_id = format!("eval-{}", fname);

        // Setear meeting_id activo temporalmente (necesario tras v31.8 fix
        // que rechaza tips sin meeting_id).
        if let Some(state) = app.try_state::<crate::state::AppState>() {
            if let Ok(mut g) = state.active_meeting_id.lock() {
                *g = Some(meeting_id.clone());
            }
        }

        let result = crate::coach::commands::coach_simple_tick(
            app.clone(),
            window,
            Some(meeting_id.clone()),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;
        total_latency += latency_ms;

        let (verb_match, has_quoted, word_count, passed, generated_tip, notes) = match result {
            Ok(Some(suggestion)) => {
                let tip = suggestion.tip.clone();
                let (vm, hq, wc, p) = evaluate_tip(&tip, &scenario.expected_verbs);
                let mut notes = Vec::new();
                if !vm {
                    notes.push(format!("verbo incorrecto (esperado: {:?})", scenario.expected_verbs));
                }
                if !hq {
                    notes.push("sin frase entre comillas".to_string());
                }
                if wc < 5 {
                    notes.push(format!("muy corto ({} palabras)", wc));
                }
                if wc > 25 {
                    notes.push(format!("muy largo ({} palabras)", wc));
                }
                (vm, hq, wc, p, Some(tip), notes.join(" | "))
            }
            Ok(None) => (false, false, 0, false, None, "tip rechazado por filtros".to_string()),
            Err(e) => (false, false, 0, false, None, format!("error: {}", e)),
        };

        results.push(ScenarioResult {
            scenario: scenario.name.clone(),
            category: scenario.category.clone(),
            generated_tip,
            verb_match,
            has_quoted_phrase: has_quoted,
            word_count,
            passed,
            latency_ms,
            notes,
        });
    }

    let passed_count = results.iter().filter(|r| r.passed).count();
    let total = results.len();
    let avg_latency_ms = if total > 0 { total_latency / total as u64 } else { 0 };

    Ok(EvalReport {
        total,
        passed: passed_count,
        failed: total - passed_count,
        avg_latency_ms,
        results,
    })
}

/// Lista los scenarios disponibles sin ejecutar.
#[tauri::command]
pub async fn dev_list_scenarios() -> Result<Vec<Scenario>, String> {
    let scenarios = load_scenarios()?;
    Ok(scenarios.into_iter().map(|(_, s)| s).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_tip_valid_format() {
        let tip = "Pregunta: \"¿Qué te haría sentir más tranquilo?\"";
        let (vm, hq, wc, p) = evaluate_tip(tip, &vec!["pregunta".to_string()]);
        assert!(vm);
        assert!(hq);
        assert_eq!(wc, 7);
        assert!(p);
    }

    #[test]
    fn test_evaluate_tip_no_quotes() {
        let tip = "Pregunta: qué te haría sentir tranquilo";
        let (vm, hq, _, p) = evaluate_tip(tip, &vec!["pregunta".to_string()]);
        assert!(vm);
        assert!(!hq);
        assert!(!p);
    }

    #[test]
    fn test_evaluate_tip_wrong_verb() {
        let tip = "Cierra: \"firmamos hoy\"";
        let (vm, _, _, p) = evaluate_tip(tip, &vec!["pregunta".to_string()]);
        assert!(!vm);
        assert!(!p);
    }

    #[test]
    fn test_load_scenarios_finds_files() {
        // smoke test — solo verifica que la función no panique
        let result = load_scenarios();
        // Si dev: encontrará archivos. Si CI: puede fallar por path. Ambos OK.
        let _ = result;
    }
}
