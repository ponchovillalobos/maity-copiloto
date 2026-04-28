//! Compliance audit module.
//! Genera audit logs criptográficos para reportar a equipos legal/compliance
//! que demuestran 100% local execution sin egreso de datos.

pub mod audit_log;
pub mod report;
pub mod commands;

// Direct re-export of commands for Tauri handler registration
pub use commands::{
    compliance_log_event, compliance_get_meeting_audit, compliance_export_report,
    ComplianceEvent, ComplianceMeetingAudit, ComplianceExportResult,
};
