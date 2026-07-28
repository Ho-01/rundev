CREATE TABLE IF NOT EXISTS keyboard_daily_stats (
    local_date TEXT PRIMARY KEY,
    press_count INTEGER NOT NULL DEFAULT 0 CHECK (press_count >= 0),
    rewarded_milestones INTEGER NOT NULL DEFAULT 0 CHECK (rewarded_milestones >= 0),
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS daily_activity_metrics (
    local_date TEXT NOT NULL,
    metric_type TEXT NOT NULL,
    source TEXT NOT NULL,
    value INTEGER NOT NULL DEFAULT 0 CHECK (value >= 0),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (local_date, metric_type, source)
);

CREATE INDEX IF NOT EXISTS idx_daily_activity_metrics_date
    ON daily_activity_metrics(local_date);

