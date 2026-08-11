CREATE TABLE IF NOT EXISTS character_skin_ownership (
    runner_id TEXT NOT NULL,
    skin_id TEXT NOT NULL,
    unlocked_at TEXT NOT NULL,
    unlock_source TEXT NOT NULL,
    PRIMARY KEY (runner_id, skin_id)
);

CREATE TABLE IF NOT EXISTS character_skin_loadout (
    runner_id TEXT PRIMARY KEY,
    skin_id TEXT NOT NULL,
    equipped_at TEXT NOT NULL
);
