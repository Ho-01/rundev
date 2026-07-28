ALTER TABLE ai_usage_events ADD COLUMN session_key TEXT;

CREATE INDEX IF NOT EXISTS idx_ai_usage_session_activity
    ON ai_usage_events(provider, session_key, occurred_at)
    WHERE session_key IS NOT NULL;
