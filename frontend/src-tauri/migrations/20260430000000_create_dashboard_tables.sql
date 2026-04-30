-- Maity v0.4.0 — Dashboard milimétrico: tracking de iteraciones de testing
-- (audios cargados via /dev) y matriz de auditoría manual de botones.

CREATE TABLE IF NOT EXISTS dev_iterations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    iteration_label TEXT,
    audio_user_path TEXT,
    audio_interlocutor_path TEXT,
    channel_layout TEXT,
    total_duration_seconds REAL,
    decode_ms INTEGER,
    transcribe_user_ms INTEGER,
    transcribe_interlocutor_ms INTEGER,
    evaluation_ms INTEGER,
    total_pipeline_ms INTEGER,
    wer_global REAL,
    wer_user REAL,
    wer_interlocutor REAL,
    hypothesis_full TEXT,
    reference_user TEXT,
    reference_interlocutor TEXT,
    evaluation_score REAL,
    evaluation_sections_filled INTEGER,
    prompt_version TEXT NOT NULL DEFAULT 'v3-lite',
    coach_model TEXT NOT NULL DEFAULT 'qwen3:0.6b',
    evaluation_model TEXT NOT NULL DEFAULT 'qwen3:1.7b',
    cpu_avg_pct REAL,
    ram_peak_mb INTEGER,
    notes TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_dev_iter_created ON dev_iterations(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_dev_iter_meeting ON dev_iterations(meeting_id);

CREATE TABLE IF NOT EXISTS button_audit (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    source_file TEXT NOT NULL,
    source_line INTEGER,
    category TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'untested',
    notes TEXT,
    last_checked_at DATETIME,
    last_checked_iteration_id INTEGER REFERENCES dev_iterations(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_button_category ON button_audit(category);
CREATE INDEX IF NOT EXISTS idx_button_status ON button_audit(status);
