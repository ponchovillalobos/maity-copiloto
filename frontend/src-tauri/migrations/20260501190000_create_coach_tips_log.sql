-- v26: histórico permanente de tips generados en producción.
-- Diferente de tip_tests (que es para test con ground truth).

CREATE TABLE IF NOT EXISTS coach_tips_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT,
    tip TEXT NOT NULL,
    category TEXT,
    subcategory TEXT,
    technique TEXT,
    priority TEXT,
    tip_type TEXT,
    confidence REAL,
    latency_ms INTEGER,
    model TEXT,
    minute INTEGER,
    trigger_signal TEXT,
    suggested_category TEXT,
    is_duplicate INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_coach_tips_log_created ON coach_tips_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_coach_tips_log_meeting ON coach_tips_log(meeting_id);
