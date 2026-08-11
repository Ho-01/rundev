use serde::Serialize;
use sqlx::SqlitePool;

pub const DEFAULT_SKIN_ID: &str = "default";
pub const POOL_PARTY_SKIN_ID: &str = "pool-party";
const POOL_PARTY_REQUIRED_ACTIVE_SECONDS: i64 = 5 * 60 * 60;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSelection {
    pub runner_id: String,
    pub skin_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSkinCollection {
    pub selected: RunnerSelection,
    pub total_development_seconds: i64,
    pub characters: Vec<RunnerCharacter>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerCharacter {
    pub runner_id: String,
    pub name: String,
    pub skins: Vec<RunnerSkin>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSkin {
    pub skin_id: String,
    pub name: String,
    pub description: String,
    pub required_active_seconds: i64,
    pub owned: bool,
    pub equipped: bool,
}

#[derive(Clone, Copy)]
struct RunnerDefinition {
    runner_id: &'static str,
    name: &'static str,
}

#[derive(Clone, Copy)]
struct SkinDefinition {
    runner_id: &'static str,
    skin_id: &'static str,
    name: &'static str,
    description: &'static str,
    required_active_seconds: i64,
}

const RUNNERS: &[RunnerDefinition] = &[
    RunnerDefinition {
        runner_id: "coding-cat",
        name: "코딩 고양이",
    },
    RunnerDefinition {
        runner_id: "coding-orange-cat",
        name: "주황 고양이",
    },
    RunnerDefinition {
        runner_id: "coding-shrimp",
        name: "주황 새우",
    },
    RunnerDefinition {
        runner_id: "coding-fish",
        name: "파란 물고기",
    },
    RunnerDefinition {
        runner_id: "coding-vtuber",
        name: "핑크 버튜버",
    },
];

const SKINS: &[SkinDefinition] = &[
    SkinDefinition {
        runner_id: "coding-cat",
        skin_id: DEFAULT_SKIN_ID,
        name: "기본 스킨",
        description: "RunDev의 기본 코딩 고양이입니다.",
        required_active_seconds: 0,
    },
    SkinDefinition {
        runner_id: "coding-orange-cat",
        skin_id: DEFAULT_SKIN_ID,
        name: "기본 스킨",
        description: "RunDev의 기본 주황 고양이입니다.",
        required_active_seconds: 0,
    },
    SkinDefinition {
        runner_id: "coding-shrimp",
        skin_id: DEFAULT_SKIN_ID,
        name: "기본 스킨",
        description: "RunDev의 기본 주황 새우입니다.",
        required_active_seconds: 0,
    },
    SkinDefinition {
        runner_id: "coding-fish",
        skin_id: DEFAULT_SKIN_ID,
        name: "기본 스킨",
        description: "RunDev의 기본 파란 물고기입니다.",
        required_active_seconds: 0,
    },
    SkinDefinition {
        runner_id: "coding-vtuber",
        skin_id: DEFAULT_SKIN_ID,
        name: "기본 스킨",
        description: "헤드셋을 쓰고 코딩하는 핑크 버튜버입니다.",
        required_active_seconds: 0,
    },
    SkinDefinition {
        runner_id: "coding-vtuber",
        skin_id: POOL_PARTY_SKIN_ID,
        name: "수영장 파티",
        description: "선글라스와 파란 도트 비키니로 여름 코딩을 즐깁니다.",
        required_active_seconds: POOL_PARTY_REQUIRED_ACTIVE_SECONDS,
    },
];

pub fn is_supported_runner(runner_id: &str) -> bool {
    RUNNERS.iter().any(|runner| runner.runner_id == runner_id)
}

fn normalize_runner_id(runner_id: String) -> String {
    if runner_id == "coding-white-cat" {
        "coding-shrimp".to_string()
    } else {
        runner_id
    }
}

fn skin_definition(runner_id: &str, skin_id: &str) -> Option<SkinDefinition> {
    SKINS
        .iter()
        .find(|skin| skin.runner_id == runner_id && skin.skin_id == skin_id)
        .copied()
}

async fn selected_runner_id(pool: &SqlitePool) -> Result<String, String> {
    let runner_id =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'runner.selected'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| "coding-cat".to_string());
    Ok(normalize_runner_id(runner_id))
}

async fn equipped_skin_id(pool: &SqlitePool, runner_id: &str) -> Result<String, String> {
    let skin_id: Option<String> =
        sqlx::query_scalar("SELECT skin_id FROM character_skin_loadout WHERE runner_id = ?")
            .bind(runner_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    let skin_id = skin_id.unwrap_or_else(|| DEFAULT_SKIN_ID.to_string());
    Ok(if skin_definition(runner_id, &skin_id).is_some() {
        skin_id
    } else {
        DEFAULT_SKIN_ID.to_string()
    })
}

pub async fn selection(pool: &SqlitePool) -> Result<RunnerSelection, String> {
    let runner_id = selected_runner_id(pool).await?;
    Ok(RunnerSelection {
        skin_id: equipped_skin_id(pool, &runner_id).await?,
        runner_id,
    })
}

async fn total_development_seconds(pool: &SqlitePool) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(active_seconds), 0)
         FROM activity_sessions
         WHERE activity_type = 'development'",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())
}

async fn unlock_available_skins(
    pool: &SqlitePool,
    total_active_seconds: i64,
) -> Result<(), String> {
    let unlocked_at = chrono::Utc::now().to_rfc3339();
    for skin in SKINS.iter().filter(|skin| {
        skin.required_active_seconds > 0 && total_active_seconds >= skin.required_active_seconds
    }) {
        sqlx::query(
            "INSERT OR IGNORE INTO character_skin_ownership
                (runner_id, skin_id, unlocked_at, unlock_source)
             VALUES (?, ?, ?, 'development-active-seconds')",
        )
        .bind(skin.runner_id)
        .bind(skin.skin_id)
        .bind(&unlocked_at)
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

async fn owns_skin(pool: &SqlitePool, skin: SkinDefinition) -> Result<bool, String> {
    if skin.required_active_seconds == 0 {
        return Ok(true);
    }
    let owned: Option<String> = sqlx::query_scalar(
        "SELECT skin_id FROM character_skin_ownership WHERE runner_id = ? AND skin_id = ?",
    )
    .bind(skin.runner_id)
    .bind(skin.skin_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| error.to_string())?;
    Ok(owned.is_some())
}

pub async fn collection(pool: &SqlitePool) -> Result<RunnerSkinCollection, String> {
    let total_active_seconds = total_development_seconds(pool).await?;
    unlock_available_skins(pool, total_active_seconds).await?;
    let selected = selection(pool).await?;
    let mut characters = Vec::with_capacity(RUNNERS.len());

    for runner in RUNNERS {
        let equipped_skin = equipped_skin_id(pool, runner.runner_id).await?;
        let mut skins = Vec::new();
        for skin in SKINS
            .iter()
            .filter(|skin| skin.runner_id == runner.runner_id)
        {
            skins.push(RunnerSkin {
                skin_id: skin.skin_id.to_string(),
                name: skin.name.to_string(),
                description: skin.description.to_string(),
                required_active_seconds: skin.required_active_seconds,
                owned: owns_skin(pool, *skin).await?,
                equipped: equipped_skin == skin.skin_id,
            });
        }
        characters.push(RunnerCharacter {
            runner_id: runner.runner_id.to_string(),
            name: runner.name.to_string(),
            skins,
        });
    }

    Ok(RunnerSkinCollection {
        selected,
        total_development_seconds: total_active_seconds,
        characters,
    })
}

pub async fn select_runner(pool: &SqlitePool, runner_id: &str) -> Result<RunnerSelection, String> {
    if !is_supported_runner(runner_id) {
        return Err("지원하지 않는 개발자 캐릭터입니다.".to_string());
    }
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES ('runner.selected', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(runner_id)
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    selection(pool).await
}

pub async fn equip_skin(
    pool: &SqlitePool,
    runner_id: &str,
    skin_id: &str,
) -> Result<RunnerSelection, String> {
    let skin = skin_definition(runner_id, skin_id)
        .ok_or_else(|| "지원하지 않는 캐릭터 스킨입니다.".to_string())?;
    let total_active_seconds = total_development_seconds(pool).await?;
    unlock_available_skins(pool, total_active_seconds).await?;
    if !owns_skin(pool, skin).await? {
        return Err("이 스킨은 누적 집중 시간을 달성한 뒤 사용할 수 있습니다.".to_string());
    }
    sqlx::query(
        "INSERT INTO character_skin_loadout (runner_id, skin_id, equipped_at)
         VALUES (?, ?, ?)
         ON CONFLICT(runner_id) DO UPDATE SET
             skin_id = excluded.skin_id,
             equipped_at = excluded.equipped_at",
    )
    .bind(runner_id)
    .bind(skin_id)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|error| error.to_string())?;
    selection(pool).await
}

#[cfg(test)]
mod tests {
    use super::{is_supported_runner, skin_definition, DEFAULT_SKIN_ID, POOL_PARTY_SKIN_ID};

    #[test]
    fn accepts_only_packaged_runner_ids() {
        assert!(is_supported_runner("coding-vtuber"));
        assert!(!is_supported_runner("../custom"));
    }

    #[test]
    fn pool_party_skin_belongs_only_to_the_pink_vtuber() {
        assert!(skin_definition("coding-vtuber", POOL_PARTY_SKIN_ID).is_some());
        assert!(skin_definition("coding-cat", POOL_PARTY_SKIN_ID).is_none());
        assert!(skin_definition("coding-vtuber", DEFAULT_SKIN_ID).is_some());
    }
}
