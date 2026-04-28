//! Integración calendario: parsing local de archivos .ics (estándar iCalendar)
//! exportados por Outlook/Google Calendar. NO conecta a APIs externas — el
//! usuario importa archivos manualmente. Privacidad-first.

pub mod ics_parser;
pub mod commands;

pub use commands::{
    calendar_parse_ics_file,
    calendar_match_meeting_to_event,
    CalendarEvent,
    MatchResult,
};
