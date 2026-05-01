//! Parser de salida LLM para sugerencias de coaching.
//!
//! Tolera markdown wrapping, thinking tags y ruido alrededor del JSON.

use super::types::RawSuggestion;

/// Parsea la salida del LLM. Tolerante a markdown wrapping, thinking tags y ruido alrededor del JSON.
pub fn parse_llm_output(raw: &str) -> Result<RawSuggestion, String> {
    let cleaned = crate::coach::parse_helpers::clean_llm_output(raw);

    // Intento directo
    if let Ok(parsed) = serde_json::from_str::<RawSuggestion>(&cleaned) {
        return Ok(parsed);
    }

    // Buscar el primer { y el último } (tolerante a texto antes/después)
    let start = cleaned.find('{');
    let end = cleaned.rfind('}');
    if let (Some(s), Some(e)) = (start, end) {
        if e > s {
            let slice = &cleaned[s..=e];
            return serde_json::from_str::<RawSuggestion>(slice)
                .map_err(|err| format!("JSON inválido: {} | raw: {}", err, slice));
        }
    }

    Err(format!(
        "No se pudo parsear salida del LLM (no encontré JSON): {}",
        cleaned
    ))
}

/// Infiere `tip_type` a partir del tip + priority cuando el LLM no lo provee.
///
/// Heurística:
/// - Empieza con "Excelente/Bien/Perfecto/Gran/Buen" → recognition
/// - Empieza con "¿" → introspective
/// - Empieza con "Noto/He notado/Observo" → observation
/// - priority in {critical, important} → corrective
/// - resto → observation
pub fn infer_tip_type(tip: &str, priority: &str) -> String {
    let trimmed = tip.trim_start();
    let lower = trimmed.to_lowercase();
    const RECOG: &[&str] = &[
        "excelente", "bien hecho", "perfecto", "gran ", "buen ", "increible",
        "muy bien", "genial",
    ];
    if RECOG.iter().any(|p| lower.starts_with(p)) {
        return "recognition".to_string();
    }
    if trimmed.starts_with('¿') || trimmed.starts_with('?') {
        return "introspective".to_string();
    }
    const OBS: &[&str] = &["noto", "he notado", "observo", "veo que"];
    if OBS.iter().any(|p| lower.starts_with(p)) {
        return "observation".to_string();
    }
    match priority {
        "critical" | "important" => "corrective".to_string(),
        _ => "observation".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directo() {
        let raw = r#"{"tip":"Hola","category":"rapport","confidence":0.8}"#;
        let result = parse_llm_output(raw).unwrap();
        assert_eq!(result.tip, "Hola");
        assert_eq!(result.category, "rapport");
        assert!((result.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_parse_con_markdown() {
        let raw = "```json\n{\"tip\":\"Pregunta sobre el fin de semana\",\"category\":\"icebreaker\",\"confidence\":0.7}\n```";
        let result = parse_llm_output(raw).unwrap();
        assert_eq!(result.category, "icebreaker");
    }

    #[test]
    fn test_parse_con_ruido_alrededor() {
        let raw = r#"Aquí va mi respuesta: {"tip":"Cierra ahora","category":"closing","confidence":0.95} Espero ayude."#;
        let result = parse_llm_output(raw).unwrap();
        assert_eq!(result.tip, "Cierra ahora");
    }

    #[test]
    fn test_parse_invalido() {
        assert!(parse_llm_output("texto sin json").is_err());
    }
}
