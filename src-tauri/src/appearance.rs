use sqlx::SqlitePool;

const KEY: &str = "appearance.accent_theme";
pub const DEFAULT_THEME: &str = "default";
const THEMES: &[&str] = &[
    "default",
    "periwinkle",
    "lavender",
    "sky-blue",
    "ice-cyan",
    "steel-blue",
    "silver",
    "champagne",
    "honey-amber",
    "peach",
    "coral",
    "rose",
    "orchid",
];

fn supported(theme: &str) -> bool {
    THEMES.contains(&theme)
}

pub async fn load(pool: &SqlitePool) -> Result<String, String> {
    let stored: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(KEY)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
    Ok(stored
        .filter(|theme| supported(theme))
        .unwrap_or_else(|| DEFAULT_THEME.into()))
}

pub async fn save(pool: &SqlitePool, theme: &str) -> Result<(), String> {
    if !supported(theme) {
        return Err("지원하지 않는 색상 테마입니다.".into());
    }
    sqlx::query("INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(KEY).bind(theme).execute(pool).await.map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persists_themes_and_rejects_invalid_values_without_losing_the_selection() {
        let pool = crate::database::connect("sqlite::memory:").await.unwrap();
        assert_eq!(load(&pool).await.unwrap(), DEFAULT_THEME);
        for theme in THEMES {
            save(&pool, theme).await.unwrap();
            assert_eq!(load(&pool).await.unwrap(), *theme);
        }
        assert!(save(&pool, "#ff0000").await.is_err());
        assert_eq!(load(&pool).await.unwrap(), "orchid");
        sqlx::query("UPDATE app_settings SET value = 'unknown' WHERE key = ?")
            .bind(KEY)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(load(&pool).await.unwrap(), DEFAULT_THEME);
    }
}
