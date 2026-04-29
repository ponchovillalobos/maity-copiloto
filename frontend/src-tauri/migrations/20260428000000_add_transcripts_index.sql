-- Optimización detectada por la auditoría de performance:
-- La tabla `transcripts` se consulta SIEMPRE por `meeting_id` (Sidebar reload,
-- evaluación post-meeting, chat con reunión, búsqueda semántica). Sin índice
-- es full table scan O(n) → degrada con 10k+ registros.
--
-- Idempotente: usa IF NOT EXISTS por si la migración corre dos veces.

CREATE INDEX IF NOT EXISTS idx_transcripts_meeting_id ON transcripts(meeting_id);

-- Bonus: índice combinado para queries ordenadas por timestamp dentro de una reunión.
CREATE INDEX IF NOT EXISTS idx_transcripts_meeting_timestamp ON transcripts(meeting_id, timestamp);
