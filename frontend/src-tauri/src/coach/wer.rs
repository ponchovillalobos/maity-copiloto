//! Word Error Rate (WER) calculator para evaluar transcripción contra
//! ground truth. Estándar industria para STT.
//!
//! WER = (Insertions + Deletions + Substitutions) / N_reference_words
//!
//! Implementación: Levenshtein a nivel token, normalización español
//! (lowercase, strip puntuación, colapsar espacios, eliminar tildes opcional).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WerResult {
    /// WER (0.0 = perfecto, 1.0 = todo mal). Multiplicar por 100 para %.
    pub wer: f32,
    /// Total palabras en referencia.
    pub reference_words: usize,
    /// Total palabras en hipótesis (lo que el modelo transcribió).
    pub hypothesis_words: usize,
    pub substitutions: usize,
    pub insertions: usize,
    pub deletions: usize,
    /// Palabras correctas (matches).
    pub hits: usize,
}

/// Normaliza texto para comparación: lowercase, strip puntuación común,
/// colapsa whitespace. Mantiene tildes (importante en español).
pub fn normalize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            '.' | ',' | ';' | ':' | '!' | '¡' | '?' | '¿' | '"' | '\'' | '-' | '—' | '(' | ')'
            | '[' | ']' | '{' | '}' => ' ',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .map(|w| w.to_string())
        .collect()
}

/// Calcula WER entre referencia (ground truth) e hipótesis (transcripción
/// del modelo). Usa programación dinámica (Levenshtein editado para tokens).
pub fn compute_wer(reference: &str, hypothesis: &str) -> WerResult {
    let ref_tokens = normalize(reference);
    let hyp_tokens = normalize(hypothesis);
    let n = ref_tokens.len();
    let m = hyp_tokens.len();

    if n == 0 {
        return WerResult {
            wer: if m == 0 { 0.0 } else { 1.0 },
            reference_words: 0,
            hypothesis_words: m,
            substitutions: 0,
            insertions: m,
            deletions: 0,
            hits: 0,
        };
    }

    // dp[i][j] = edits para transformar ref[..i] en hyp[..j]
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            if ref_tokens[i - 1] == hyp_tokens[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                let sub = dp[i - 1][j - 1] + 1;
                let ins = dp[i][j - 1] + 1;
                let del = dp[i - 1][j] + 1;
                dp[i][j] = sub.min(ins).min(del);
            }
        }
    }

    // Backtrace para contar S/I/D separadamente
    let mut subs = 0usize;
    let mut inss = 0usize;
    let mut dels = 0usize;
    let mut hits = 0usize;
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && ref_tokens[i - 1] == hyp_tokens[j - 1] {
            hits += 1;
            i -= 1;
            j -= 1;
        } else if i > 0 && j > 0 && dp[i][j] == dp[i - 1][j - 1] + 1 {
            subs += 1;
            i -= 1;
            j -= 1;
        } else if j > 0 && dp[i][j] == dp[i][j - 1] + 1 {
            inss += 1;
            j -= 1;
        } else if i > 0 && dp[i][j] == dp[i - 1][j] + 1 {
            dels += 1;
            i -= 1;
        } else {
            break;
        }
    }

    let total_errors = subs + inss + dels;
    let wer = total_errors as f32 / n as f32;

    WerResult {
        wer,
        reference_words: n,
        hypothesis_words: m,
        substitutions: subs,
        insertions: inss,
        deletions: dels,
        hits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_punct() {
        let words = normalize("Hola, ¿cómo estás?");
        assert_eq!(words, vec!["hola", "cómo", "estás"]);
    }

    #[test]
    fn test_perfect_match() {
        let r = compute_wer("hola mundo", "hola mundo");
        assert_eq!(r.wer, 0.0);
        assert_eq!(r.hits, 2);
        assert_eq!(r.substitutions, 0);
    }

    #[test]
    fn test_one_substitution() {
        let r = compute_wer("hola mundo", "hola tierra");
        assert_eq!(r.substitutions, 1);
        assert_eq!(r.hits, 1);
        assert!((r.wer - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_insertion() {
        let r = compute_wer("hola mundo", "hola gran mundo");
        assert_eq!(r.insertions, 1);
        assert_eq!(r.hits, 2);
        assert!((r.wer - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_deletion() {
        let r = compute_wer("hola gran mundo", "hola mundo");
        assert_eq!(r.deletions, 1);
        assert_eq!(r.hits, 2);
        assert!((r.wer - (1.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn test_empty_reference() {
        let r = compute_wer("", "hola");
        assert_eq!(r.wer, 1.0);
        assert_eq!(r.insertions, 1);
    }

    #[test]
    fn test_both_empty() {
        let r = compute_wer("", "");
        assert_eq!(r.wer, 0.0);
    }

    #[test]
    fn test_full_mismatch() {
        let r = compute_wer("uno dos tres", "cuatro cinco seis");
        assert_eq!(r.wer, 1.0);
        assert_eq!(r.substitutions, 3);
    }

    #[test]
    fn test_punct_does_not_count() {
        let r = compute_wer("Hola, mundo.", "Hola mundo");
        assert_eq!(r.wer, 0.0);
    }
}
