CREATE TABLE IF NOT EXISTS whip_daily_stats (
    local_date TEXT PRIMARY KEY,
    whip_count INTEGER NOT NULL DEFAULT 0 CHECK (whip_count >= 0),
    updated_at TEXT NOT NULL
);
