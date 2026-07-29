CREATE TABLE IF NOT EXISTS cursor_usage_snapshots (
    id TEXT PRIMARY KEY,
    account_key TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    cycle_started_at TEXT,
    cycle_ends_at TEXT,
    plan_kind TEXT,
    used_microusd INTEGER,
    limit_microusd INTEGER,
    remaining_microusd INTEGER,
    auto_percent_basis_points INTEGER,
    api_percent_basis_points INTEGER,
    today_microusd INTEGER,
    total_tokens INTEGER,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cached_tokens INTEGER,
    source TEXT NOT NULL,
    confidence TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cursor_usage_account_observed
    ON cursor_usage_snapshots(account_key, observed_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cursor_usage_dedupe
    ON cursor_usage_snapshots(
        account_key,
        COALESCE(cycle_started_at, ''),
        COALESCE(used_microusd, -1),
        COALESCE(limit_microusd, -1),
        COALESCE(today_microusd, -1),
        COALESCE(total_tokens, -1)
    );
