CREATE TABLE IF NOT EXISTS character_traits (
    trait_id TEXT PRIMARY KEY CHECK (
        trait_id IN ('focus-ready', 'hot-keyboard', 'reload', 'context-runner')
    ),
    level INTEGER NOT NULL DEFAULT 0 CHECK (level BETWEEN 0 AND 20)
);

INSERT OR IGNORE INTO character_traits (trait_id, level) VALUES
    ('focus-ready', 0),
    ('hot-keyboard', 0),
    ('reload', 0),
    ('context-runner', 0);

CREATE TABLE IF NOT EXISTS trait_bonus_accumulators (
    trait_id TEXT PRIMARY KEY,
    basis_point_xp INTEGER NOT NULL DEFAULT 0 CHECK (basis_point_xp >= 0)
);
