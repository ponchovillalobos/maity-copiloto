//! Mini-parser iCalendar (.ics) RFC 5545 simplificado.
//! Solo extrae VEVENT con campos esenciales: SUMMARY, DESCRIPTION, DTSTART,
//! DTEND, ATTENDEE. Ignora recurrencia compleja (RRULE), zonas horarias TZID
//! avanzadas, y tareas (VTODO). Suficiente para mapear reuniones de Outlook
//! a sesiones de Maity.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParsedEvent {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub start: String,         // ISO 8601 simplificado (yyyymmddThhmmssZ del ics)
    pub end: String,
    pub organizer: Option<String>,
    pub attendees: Vec<String>,
    pub location: Option<String>,
}

/// Parsea contenido .ics y devuelve lista de VEVENTs.
pub fn parse_ics(content: &str) -> Vec<ParsedEvent> {
    let mut events = Vec::new();
    let mut current: Option<ParsedEvent> = None;

    // Unfolding RFC 5545: las líneas que empiezan con espacio o tab son
    // continuación de la línea anterior. Procesamos primero.
    let mut unfolded = String::new();
    for line in content.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            unfolded.push_str(&line[1..]);
        } else {
            if !unfolded.is_empty() {
                unfolded.push('\n');
            }
            unfolded.push_str(line);
        }
    }

    for line in unfolded.lines() {
        let line = line.trim_end();
        if line == "BEGIN:VEVENT" {
            current = Some(ParsedEvent::default());
            continue;
        }
        if line == "END:VEVENT" {
            if let Some(ev) = current.take() {
                events.push(ev);
            }
            continue;
        }
        if let Some(ref mut ev) = current {
            // Buscar el separador `:` después del nombre de propiedad.
            // Ejemplos:
            //   SUMMARY:Mi reunión
            //   DTSTART;TZID=America/Mexico_City:20260427T100000
            //   ATTENDEE;CN=Cliente:mailto:cliente@example.com
            if let Some(colon_idx) = line.find(':') {
                let prop_part = &line[..colon_idx];
                let value = unescape_ics(&line[colon_idx + 1..]);
                let prop_name = prop_part.split(';').next().unwrap_or("").to_uppercase();
                match prop_name.as_str() {
                    "UID" => ev.uid = value,
                    "SUMMARY" => ev.summary = value,
                    "DESCRIPTION" => ev.description = value,
                    "DTSTART" => ev.start = value,
                    "DTEND" => ev.end = value,
                    "LOCATION" => ev.location = Some(value),
                    "ORGANIZER" => ev.organizer = Some(strip_mailto(&value)),
                    "ATTENDEE" => ev.attendees.push(strip_mailto(&value)),
                    _ => {}
                }
            }
        }
    }

    events
}

fn unescape_ics(s: &str) -> String {
    // RFC 5545: \\ → \  \, → ,  \; → ;  \n → newline
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn strip_mailto(s: &str) -> String {
    s.strip_prefix("mailto:")
        .or_else(|| s.strip_prefix("MAILTO:"))
        .map(|x| x.to_string())
        .unwrap_or_else(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_event() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:abc-123
SUMMARY:Reunión cliente Acme
DTSTART:20260427T100000Z
DTEND:20260427T110000Z
ORGANIZER:mailto:me@example.com
ATTENDEE:mailto:cliente@acme.com
END:VEVENT
END:VCALENDAR"#;
        let events = parse_ics(ics);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].uid, "abc-123");
        assert_eq!(events[0].summary, "Reunión cliente Acme");
        assert_eq!(events[0].organizer.as_deref(), Some("me@example.com"));
        assert_eq!(events[0].attendees.len(), 1);
        assert_eq!(events[0].attendees[0], "cliente@acme.com");
    }

    #[test]
    fn handles_escapes_in_description() {
        let ics = "BEGIN:VEVENT\nUID:1\nSUMMARY:Test\nDESCRIPTION:Linea1\\nLinea2\\, con coma\nEND:VEVENT";
        let events = parse_ics(ics);
        assert_eq!(events[0].description, "Linea1\nLinea2, con coma");
    }

    #[test]
    fn handles_multiple_events() {
        let ics = "BEGIN:VEVENT\nUID:a\nSUMMARY:A\nEND:VEVENT\nBEGIN:VEVENT\nUID:b\nSUMMARY:B\nEND:VEVENT";
        let events = parse_ics(ics);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn line_unfolding_works() {
        let ics = "BEGIN:VEVENT\nUID:1\nSUMMARY:Tema importante\n con continuación\nEND:VEVENT";
        let events = parse_ics(ics);
        assert!(events[0].summary.contains("continuación"));
    }

    #[test]
    fn ignores_unknown_properties() {
        let ics = "BEGIN:VEVENT\nUID:1\nFOO:bar\nSUMMARY:Reunión\nEND:VEVENT";
        let events = parse_ics(ics);
        assert_eq!(events[0].summary, "Reunión");
    }
}
