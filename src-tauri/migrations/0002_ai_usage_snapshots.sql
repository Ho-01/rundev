CREATE TABLE IF NOT EXISTS ai_usage_snapshots (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    source TEXT NOT NULL,
    scope TEXT NOT NULL,
    bucket_started_at TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
    confidence TEXT NOT NULL,
    UNIQUE (provider, source, scope, bucket_started_at, total_tokens)
);

CREATE TABLE IF NOT EXISTS ai_adapter_state (
    adapter_id TEXT PRIMARY KEY,
    cursor TEXT,
    last_success_at TEXT,
    last_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_ai_usage_snapshots_bucket
    ON ai_usage_snapshots(provider, bucket_started_at, observed_at);

