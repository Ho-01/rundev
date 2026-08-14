-- The rebalanced point economy derives earned points from character level.
-- Reset invested levels so every existing user receives a full, automatic respec.
UPDATE character_traits SET level = 0;
DELETE FROM trait_bonus_accumulators;

CREATE TABLE IF NOT EXISTS trait_daily_rewards (
    trait_id TEXT NOT NULL CHECK (trait_id = 'reload'),
    local_date TEXT NOT NULL,
    awarded_at TEXT NOT NULL,
    basis_point_xp INTEGER NOT NULL CHECK (basis_point_xp >= 0),
    xp_awarded INTEGER NOT NULL CHECK (xp_awarded >= 0),
    PRIMARY KEY (trait_id, local_date)
);
