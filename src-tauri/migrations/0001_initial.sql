CREATE TABLE IF NOT EXISTS activity_sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    active_seconds INTEGER NOT NULL DEFAULT 0,
    activity_type TEXT NOT NULL,
    source TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_usage_events (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cached_tokens INTEGER,
    source TEXT NOT NULL,
    confidence TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS xp_events (
    id TEXT PRIMARY KEY,
    occurred_at TEXT NOT NULL,
    event_type TEXT NOT NULL,
    amount INTEGER NOT NULL,
    source_event_id TEXT UNIQUE
);

CREATE TABLE IF NOT EXISTS character_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    level INTEGER NOT NULL,
    total_xp INTEGER NOT NULL,
    current_form TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO character_state (id, level, total_xp, current_form)
VALUES (1, 1, 0, 'sprout');

CREATE INDEX IF NOT EXISTS idx_activity_started_at ON activity_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_ai_usage_occurred_at ON ai_usage_events(occurred_at);
CREATE INDEX IF NOT EXISTS idx_xp_occurred_at ON xp_events(occurred_at);
