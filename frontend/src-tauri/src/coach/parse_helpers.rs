//! Helpers compartidos para parsear salidas LLM (tips + evaluación).
//!
//! Qwen3 emite `<think>...</think>` cuando el modo thinking residual se cuela.
//! Strip idempotente + markdown fence cleanup.

/// Elimina TODOS los bloques `<think>...</think>` de Qwen3.
/// Idempotente: aplicar 2 veces da mismo resultado.
pub fn strip_qwen3_thinking(input: &str) -> String {
    let mut out = input.to_string();
    while let Some(start) = out.find("<think>") {
        if let Some(end_rel) = out[start..].find("</think>") {
            let end = start + end_rel + "</think>".len();
            out.replace_range(start..end, "");
        } else {
            // Tag abierto sin cerrar: cortar desde <think> al final.
            out.truncate(start);
            break;
        }
    }
    out
}

/// Elimina fences markdown ```json ... ``` o ``` ... ```.
pub fn strip_markdown_fences(input: &str) -> String {
    let mut s = input.trim().to_string();
    if let Some(stripped) = s.strip_prefix("```json") {
        s = stripped.trim_start().to_string();
    } else if let Some(stripped) = s.strip_prefix("```") {
        s = stripped.trim_start().to_string();
    }
    if let Some(stripped) = s.strip_suffix("```") {
        s = stripped.trim_end().to_string();
    }
    s.trim().to_string()
}

/// Pipeline completo: strip think tags + markdown fences + trim.
pub fn clean_llm_output(input: &str) -> String {
    let no_think = strip_qwen3_thinking(input);
    strip_markdown_fences(&no_think)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_simple_think() {
        let r = strip_qwen3_thinking("<think>razonando</think>{\"tip\":\"hola\"}");
        assert_eq!(r, "{\"tip\":\"hola\"}");
    }

    #[test]
    fn test_strip_multiple_think() {
        let r = strip_qwen3_thinking("<think>a</think>texto<think>b</think>fin");
        assert_eq!(r, "textofin");
    }

    #[test]
    fn test_strip_no_think() {
        let r = strip_qwen3_thinking("{\"tip\":\"hola\"}");
        assert_eq!(r, "{\"tip\":\"hola\"}");
    }

    #[test]
    fn test_strip_idempotent() {
        let once = strip_qwen3_thinking("<think>x</think>JSON");
        let twice = strip_qwen3_thinking(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn test_strip_unterminated_think() {
        let r = strip_qwen3_thinking("<think>se cortó");
        assert_eq!(r, "");
    }

    #[test]
    fn test_strip_markdown_json_fence() {
        let r = strip_markdown_fences("```json\n{\"a\":1}\n```");
        assert_eq!(r, "{\"a\":1}");
    }

    #[test]
    fn test_strip_markdown_plain_fence() {
        let r = strip_markdown_fences("```\n{\"a\":1}\n```");
        assert_eq!(r, "{\"a\":1}");
    }

    #[test]
    fn test_clean_pipeline() {
        let r = clean_llm_output("<think>pensando</think>\n```json\n{\"tip\":\"x\"}\n```");
        assert_eq!(r, "{\"tip\":\"x\"}");
    }

    #[test]
    fn test_clean_only_think() {
        let r = clean_llm_output("<think>solo razonamiento</think>");
        assert_eq!(r, "");
    }

    #[test]
    fn test_clean_only_json() {
        let r = clean_llm_output("{\"tip\":\"directo\"}");
        assert_eq!(r, "{\"tip\":\"directo\"}");
    }
}
