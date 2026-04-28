//! Generación de reporte compliance: hash transcripción + análisis eventos.

use crate::database::repositories::meeting::MeetingsRepository;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use super::audit_log::{get_events_for_meeting, AuditEvent};

pub struct ComplianceData {
    pub meeting_id: String,
    pub transcript_hash: String,       // SHA-256 hex
    pub transcript_chars: usize,
    pub events: Vec<AuditEvent>,
    pub external_endpoints_detected: Vec<String>, // empty si todo local
    pub local_endpoints_used: Vec<String>,
}

pub async fn build_compliance_data(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<ComplianceData, String> {
    // Fetch all transcripts with high limit to get everything
    let (transcripts, _total) = MeetingsRepository::get_meeting_transcripts_paginated(pool, meeting_id, 100000, 0)
        .await
        .map_err(|e| format!("Error cargando transcripts: {}", e))?;

    let combined: String = transcripts
        .iter()
        .map(|t| t.transcript.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let events = get_events_for_meeting(pool, meeting_id)
        .await
        .map_err(|e| format!("Error cargando audit log: {}", e))?;

    let mut external: Vec<String> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    for ev in &events {
        if let Some(ep) = ev.endpoint.as_deref() {
            if ep.contains("localhost") || ep.contains("127.0.0.1") || ep.starts_with("file://") {
                if !local.contains(&ep.to_string()) {
                    local.push(ep.to_string());
                }
            } else {
                if !external.contains(&ep.to_string()) {
                    external.push(ep.to_string());
                }
            }
        }
    }

    Ok(ComplianceData {
        meeting_id: meeting_id.to_string(),
        transcript_hash: hash,
        transcript_chars: combined.len(),
        events,
        external_endpoints_detected: external,
        local_endpoints_used: local,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_consistency() {
        let mut h1 = Sha256::new();
        h1.update(b"hello world");
        let r1 = hex::encode(h1.finalize());
        let mut h2 = Sha256::new();
        h2.update(b"hello world");
        let r2 = hex::encode(h2.finalize());
        assert_eq!(r1, r2);
    }

    #[test]
    fn hash_different_content() {
        let mut h1 = Sha256::new();
        h1.update(b"hello world");
        let r1 = hex::encode(h1.finalize());
        let mut h2 = Sha256::new();
        h2.update(b"hello world2");
        let r2 = hex::encode(h2.finalize());
        assert_ne!(r1, r2);
    }

    #[test]
    fn endpoint_classification() {
        let local_eps = vec![
            "http://localhost:11434",
            "http://127.0.0.1:11434",
            "file:///tmp/data.txt",
        ];
        let external_eps = vec![
            "http://api.openai.com",
            "https://api.groq.com",
            "http://example.com:8080",
        ];

        for ep in local_eps {
            assert!(
                ep.contains("localhost") || ep.contains("127.0.0.1") || ep.starts_with("file://"),
                "Endpoint {} should be classified as local",
                ep
            );
        }

        for ep in external_eps {
            assert!(
                !ep.contains("localhost") && !ep.contains("127.0.0.1") && !ep.starts_with("file://"),
                "Endpoint {} should be classified as external",
                ep
            );
        }
    }
}
