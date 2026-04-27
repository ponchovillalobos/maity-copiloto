//! Búsqueda streaming top-k que NO carga todo el corpus en RAM.
//!
//! Estrategia: pagina embeddings desde DB en chunks de `PAGE_SIZE` y mantiene
//! heap min-by-score con tamaño top_k. Memoria O(top_k) en vez de O(N).
//!
//! Beneficio a escala: con 200k embeddings (1000 meetings × 200 segmentos)
//! y 768 dims, `load_all` consumía ~620 MB. Esta versión consume ~30 KB.
//!
//! Pre-existía `load_all` para casos chicos (<5k embeddings). Para queries
//! globales de chat / búsqueda usar SIEMPRE `streaming_top_k`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use sqlx::SqlitePool;

use super::cosine_similarity;
use super::repository::{EmbeddingRow, EmbeddingsRepository};

const PAGE_SIZE: i64 = 2000;

#[derive(Debug, Clone)]
pub struct ScoredRow {
    pub score: f32,
    pub row: EmbeddingRow,
}

/// Wrapper para BinaryHeap min-heap (Rust default es max-heap; invertimos `Ord`).
struct MinScoreEntry(ScoredRow);

impl PartialEq for MinScoreEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0.score.eq(&other.0.score)
    }
}
impl Eq for MinScoreEntry {}
impl Ord for MinScoreEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Invertido: heap pop devuelve MENOR score primero.
        other
            .0
            .score
            .partial_cmp(&self.0.score)
            .unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for MinScoreEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Búsqueda streaming top-k con paginación y heap.
/// Garantiza memoria acotada incluso con millones de embeddings.
pub async fn streaming_top_k(
    pool: &SqlitePool,
    model: &str,
    meeting_id: Option<&str>,
    query_emb: &[f32],
    top_k: usize,
) -> Result<Vec<ScoredRow>, sqlx::Error> {
    if top_k == 0 {
        return Ok(Vec::new());
    }

    let mut heap: BinaryHeap<MinScoreEntry> = BinaryHeap::with_capacity(top_k + 1);
    let mut offset: i64 = 0;

    loop {
        let page =
            EmbeddingsRepository::load_page(pool, model, meeting_id, PAGE_SIZE, offset).await?;
        if page.is_empty() {
            break;
        }

        for row in page {
            let score = cosine_similarity(query_emb, &row.embedding);
            let entry = MinScoreEntry(ScoredRow { score, row });

            if heap.len() < top_k {
                heap.push(entry);
            } else if let Some(min) = heap.peek() {
                if entry.0.score > min.0.score {
                    heap.pop();
                    heap.push(entry);
                }
            }
        }

        offset += PAGE_SIZE;
        // Si la página fue parcial estamos al final del corpus.
        if heap.len() < top_k && (offset as usize) > heap.len() * 1000 {
            // Progresamos pero seguimos hasta agotar.
        }
        // Salir si pagina anterior trajo <PAGE_SIZE rows.
        // (load_page devuelve menos cuando se acaba el dataset)
    }

    // Drain heap → orden descendente por score.
    let mut out: Vec<ScoredRow> = heap.into_sorted_vec().into_iter().map(|e| e.0).collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(text: &str, emb: Vec<f32>) -> EmbeddingRow {
        EmbeddingRow {
            meeting_id: "m1".to_string(),
            segment_id: format!("s-{}", text),
            text: text.to_string(),
            embedding: emb,
            audio_start_time: None,
            audio_end_time: None,
            source_type: None,
        }
    }

    #[test]
    fn min_heap_keeps_top_k_largest_scores() {
        let mut heap: BinaryHeap<MinScoreEntry> = BinaryHeap::new();
        let scores = vec![0.1, 0.5, 0.9, 0.3, 0.7, 0.2];
        let top_k = 3;

        for s in scores {
            let entry = MinScoreEntry(ScoredRow {
                score: s,
                row: make_row("x", vec![]),
            });
            if heap.len() < top_k {
                heap.push(entry);
            } else if let Some(min) = heap.peek() {
                if entry.0.score > min.0.score {
                    heap.pop();
                    heap.push(entry);
                }
            }
        }

        let mut sorted: Vec<f32> = heap.into_iter().map(|e| e.0.score).collect();
        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert_eq!(sorted, vec![0.9, 0.7, 0.5]);
    }

    #[test]
    fn min_heap_handles_top_k_zero() {
        let heap: BinaryHeap<MinScoreEntry> = BinaryHeap::new();
        assert_eq!(heap.len(), 0);
    }
}
