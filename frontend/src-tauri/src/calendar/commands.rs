//! Tauri commands para calendar.

use serde::{Deserialize, Serialize};
use std::path::Path;
use super::ics_parser::{parse_ics, ParsedEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub start: String,
    pub end: String,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
}

impl From<ParsedEvent> for CalendarEvent {
    fn from(p: ParsedEvent) -> Self {
        Self {
            uid: p.uid,
            summary: p.summary,
            description: p.description,
            start: p.start,
            end: p.end,
            organizer: p.organizer,
            attendees: p.attendees,
            location: p.location,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub event_uid: String,
    pub event_summary: String,
    pub similarity: f32, // 0.0-1.0
}

#[tauri::command]
pub async fn calendar_parse_ics_file(path: String) -> Result<Vec<CalendarEvent>, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Archivo no encontrado: {}", path));
    }
    let content = std::fs::read_to_string(p).map_err(|e| format!("Lectura: {}", e))?;
    let events = parse_ics(&content)
        .into_iter()
        .map(CalendarEvent::from)
        .collect();
    Ok(events)
}

/// Matches por similitud de título (fuzzy ascii lowercase tokens).
/// Útil para asociar reunión grabada con evento de calendario por nombre.
#[tauri::command]
pub async fn calendar_match_meeting_to_event(
    meeting_title: String,
    events: Vec<CalendarEvent>,
) -> Result<Vec<MatchResult>, String> {
    let mt = meeting_title.to_lowercase();
    let mt_tokens: std::collections::HashSet<&str> =
        mt.split_whitespace().filter(|t| t.len() > 2).collect();

    let mut matches: Vec<MatchResult> = events
        .iter()
        .map(|e| {
            let summary = e.summary.to_lowercase();
            let summary_tokens: std::collections::HashSet<&str> =
                summary.split_whitespace().filter(|t| t.len() > 2).collect();
            let intersect = mt_tokens.intersection(&summary_tokens).count();
            let union = mt_tokens.union(&summary_tokens).count().max(1);
            let similarity = intersect as f32 / union as f32;
            MatchResult {
                event_uid: e.uid.clone(),
                event_summary: e.summary.clone(),
                similarity,
            }
        })
        .filter(|m| m.similarity > 0.0)
        .collect();
    matches.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    matches.truncate(5);
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn matches_by_token_overlap() {
        let events = vec![CalendarEvent {
            uid: "1".to_string(),
            summary: "Reunión cliente Acme corp".to_string(),
            description: String::new(),
            start: String::new(),
            end: String::new(),
            organizer: None,
            attendees: vec![],
            location: None,
        }];
        let matches = calendar_match_meeting_to_event(
            "Cliente Acme".to_string(),
            events,
        )
        .await
        .unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].similarity > 0.0);
    }

    #[tokio::test]
    async fn no_match_returns_empty() {
        let events = vec![CalendarEvent {
            uid: "1".to_string(),
            summary: "Algo totalmente distinto".to_string(),
            description: String::new(),
            start: String::new(),
            end: String::new(),
            organizer: None,
            attendees: vec![],
            location: None,
        }];
        let matches = calendar_match_meeting_to_event("Hola mundo".to_string(), events).await.unwrap();
        assert_eq!(matches.len(), 0);
    }

    #[tokio::test]
    async fn truncates_to_five_matches() {
        let events = (0..10)
            .map(|i| CalendarEvent {
                uid: format!("{}", i),
                summary: "Cliente".to_string(),
                description: String::new(),
                start: String::new(),
                end: String::new(),
                organizer: None,
                attendees: vec![],
                location: None,
            })
            .collect();
        let matches = calendar_match_meeting_to_event(
            "Cliente".to_string(),
            events,
        )
        .await
        .unwrap();
        assert_eq!(matches.len(), 5);
    }
}
