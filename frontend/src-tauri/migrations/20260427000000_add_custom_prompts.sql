-- Migration: Add custom prompts table for enterprise teams
-- Allows teams to personalize coach prompts (tips/evaluation/chat/prospecting) without code changes

CREATE TABLE IF NOT EXISTS custom_prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    purpose TEXT NOT NULL,           -- 'tips' | 'evaluation' | 'chat' | 'prospecting'
    prompt_text TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_custom_prompts_purpose_active
    ON custom_prompts(purpose, is_active);
