ALTER TABLE cursor_usage_snapshots ADD COLUMN used_requests REAL;
ALTER TABLE cursor_usage_snapshots ADD COLUMN limit_requests REAL;
ALTER TABLE cursor_usage_snapshots ADD COLUMN remaining_requests REAL;
ALTER TABLE cursor_usage_snapshots ADD COLUMN today_requests REAL;

DROP INDEX IF EXISTS idx_cursor_usage_dedupe;

CREATE UNIQUE INDEX idx_cursor_usage_dedupe
    ON cursor_usage_snapshots(
        account_key,
        COALESCE(cycle_started_at, ''),
        COALESCE(used_requests, -1),
        COALESCE(limit_requests, -1),
        COALESCE(today_requests, -1),
        COALESCE(total_tokens, -1),
        COALESCE(used_microusd, -1),
        COALESCE(today_microusd, -1)
    );
