DROP INDEX IF EXISTS idx_cursor_usage_dedupe;

CREATE UNIQUE INDEX idx_cursor_usage_dedupe
    ON cursor_usage_snapshots(
        account_key,
        COALESCE(cycle_started_at, ''),
        COALESCE(used_microusd, -1),
        COALESCE(limit_microusd, -1),
        COALESCE(today_microusd, -1),
        COALESCE(total_tokens, -1),
        COALESCE(auto_percent_basis_points, -1),
        COALESCE(api_percent_basis_points, -1)
    );
