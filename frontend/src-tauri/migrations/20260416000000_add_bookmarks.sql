-- Add bookmarks table for storing bookmarks during meetings
CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recording_id TEXT NOT NULL,
    timestamp_sec REAL NOT NULL,
    category TEXT NOT NULL DEFAULT 'important',
    note TEXT,
    segment_text TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_bookmarks_recording ON bookmarks(recording_id);
