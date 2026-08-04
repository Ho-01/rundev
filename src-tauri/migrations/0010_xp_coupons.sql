CREATE TABLE IF NOT EXISTS xp_coupon_redemptions (
    coupon_id TEXT PRIMARY KEY,
    multiplier INTEGER NOT NULL CHECK (multiplier IN (2, 3)),
    redeemed_at TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_xp_coupon_active
ON xp_coupon_redemptions(starts_at, ends_at);
