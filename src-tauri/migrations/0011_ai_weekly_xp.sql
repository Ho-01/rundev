CREATE TABLE IF NOT EXISTS ai_weekly_xp_milestones (
    provider TEXT NOT NULL CHECK (provider IN ('codex', 'claude', 'cursor')),
    week_started_on TEXT NOT NULL,
    milestone_index INTEGER NOT NULL CHECK (milestone_index > 0),
    usage_tokens INTEGER NOT NULL CHECK (usage_tokens >= 0),
    awarded_at TEXT NOT NULL,
    PRIMARY KEY (provider, week_started_on, milestone_index)
);

CREATE INDEX IF NOT EXISTS idx_ai_weekly_xp_week
ON ai_weekly_xp_milestones(week_started_on, provider);
