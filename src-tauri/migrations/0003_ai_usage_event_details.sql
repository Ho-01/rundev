ALTER TABLE ai_usage_events ADD COLUMN external_event_id TEXT;
ALTER TABLE ai_usage_events ADD COLUMN model TEXT;
ALTER TABLE ai_usage_events ADD COLUMN cache_write_input_tokens INTEGER;
ALTER TABLE ai_usage_events ADD COLUMN reasoning_output_tokens INTEGER;
ALTER TABLE ai_usage_events ADD COLUMN total_tokens INTEGER;
ALTER TABLE ai_usage_events ADD COLUMN cost_usd_micros INTEGER;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_usage_external_event
    ON ai_usage_events(provider, source, external_event_id)
    WHERE external_event_id IS NOT NULL;

