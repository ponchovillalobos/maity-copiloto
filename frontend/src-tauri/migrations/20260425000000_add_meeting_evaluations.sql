-- Maity v0.4.0 — evaluación post-meeting con Gemma 4 (~12k chars JSON estructurado).
-- Persiste el resultado completo del prompt v4 (radar 6 dim, gauge, muletillas,
-- timeline, dimensiones, por_hablante, empatia, calidad_global, recomendaciones,
-- visualizaciones) por meeting. Una evaluación por meeting (PK = meeting_id).

CREATE TABLE IF NOT EXISTS meeting_evaluations (
    meeting_id TEXT PRIMARY KEY,
    evaluation_json TEXT NOT NULL,
    model_used TEXT NOT NULL,
    prompt_version TEXT NOT NULL DEFAULT 'v4-condensado',
    puntuacion_global REAL,
    nivel TEXT,
    duration_minutes INTEGER,
    sesion_anterior_id TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (sesion_anterior_id) REFERENCES meetings(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_evaluations_score ON meeting_evaluations(puntuacion_global);
CREATE INDEX IF NOT EXISTS idx_evaluations_created ON meeting_evaluations(created_at);
