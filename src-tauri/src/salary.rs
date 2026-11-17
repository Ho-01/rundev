use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub monthly_won: i64,
    pub payday: u32,
    pub start_minute: u32,
    pub end_minute: u32,
    pub lunch_start_minute: u32,
    pub lunch_end_minute: u32,
    pub daily_minutes: u32,
    // Monday = 0, Sunday = 6.
    pub weekdays: Vec<u32>,
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=1_000_000_000_000).contains(&self.monthly_won) {
            return Err("월 급여는 1원 이상 1조 원 이하로 입력하세요.".into());
        }
        if !(1..=31).contains(&self.payday) {
            return Err("급여일은 1~31일로 입력하세요.".into());
        }
        if self.start_minute >= self.end_minute
            || self.end_minute >= 1440
            || self.lunch_start_minute < self.start_minute
            || self.lunch_start_minute > self.lunch_end_minute
            || self.lunch_end_minute > self.end_minute
        {
            return Err("같은 날의 출근 ≤ 점심 시작 ≤ 점심 종료 ≤ 퇴근 순서로 입력하세요.".into());
        }
        if self.daily_minutes == 0
            || self.daily_minutes
                != self.end_minute
                    - self.start_minute
                    - (self.lunch_end_minute - self.lunch_start_minute)
        {
            return Err(
                "하루 총근무시간은 출퇴근 사이에서 점심시간을 뺀 시간과 같아야 합니다.".into(),
            );
        }
        if self.weekdays.is_empty()
            || self.weekdays.iter().any(|day| *day > 6)
            || self
                .weekdays
                .iter()
                .enumerate()
                .any(|(i, day)| self.weekdays[..i].contains(day))
        {
            return Err("중복 없이 근무 요일을 하나 이상 선택하세요.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkStatus {
    Working,
    Lunch,
    BeforeWork,
    Finished,
    DayOff,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub cycle_start: String,
    pub next_payday: String,
    pub cycle_won: f64,
    pub today_won: f64,
    pub won_per_second: f64,
    pub working: bool,
    pub work_status: WorkStatus,
    pub seconds_until_end: Option<u32>,
    pub workdays: u32,
}

fn payday_in_month(date: NaiveDate, payday: u32) -> NaiveDate {
    (1..=payday)
        .rev()
        .find_map(|day| date.with_day(day))
        .unwrap()
}

fn worked_seconds(settings: &Settings, date: NaiveDate, now: NaiveDateTime) -> i64 {
    if !settings
        .weekdays
        .contains(&date.weekday().num_days_from_monday())
        || date > now.date()
    {
        return 0;
    }
    let seconds = if date < now.date() {
        86400
    } else {
        now.time().num_seconds_from_midnight() as i64
    };
    let elapsed =
        |start: u32, end: u32| (seconds - start as i64 * 60).clamp(0, (end - start) as i64 * 60);
    elapsed(settings.start_minute, settings.lunch_start_minute)
        + elapsed(settings.lunch_end_minute, settings.end_minute)
}

pub fn calculate(settings: &Settings, now: NaiveDateTime) -> Result<Snapshot, String> {
    settings.validate()?;
    let date = now.date();
    let this_payday = payday_in_month(date, settings.payday);
    let first = date.with_day(1).unwrap();
    let (start, end) = if date >= this_payday {
        (
            this_payday,
            payday_in_month(first + Duration::days(32), settings.payday),
        )
    } else {
        (
            payday_in_month(first - Duration::days(1), settings.payday),
            this_payday,
        )
    };
    let mut workdays = 0;
    let mut elapsed = 0;
    let mut cursor = start;
    while cursor < end {
        if settings
            .weekdays
            .contains(&cursor.weekday().num_days_from_monday())
        {
            workdays += 1;
        }
        elapsed += worked_seconds(settings, cursor, now);
        cursor += Duration::days(1);
    }
    let rate =
        settings.monthly_won as f64 / (workdays as f64 * settings.daily_minutes as f64 * 60.0);
    let second = now.time().num_seconds_from_midnight();
    let workday = settings
        .weekdays
        .contains(&date.weekday().num_days_from_monday());
    let work_status = if !workday {
        WorkStatus::DayOff
    } else if second < settings.start_minute * 60 {
        WorkStatus::BeforeWork
    } else if second >= settings.end_minute * 60 {
        WorkStatus::Finished
    } else if second >= settings.lunch_start_minute * 60 && second < settings.lunch_end_minute * 60
    {
        WorkStatus::Lunch
    } else {
        WorkStatus::Working
    };
    let working = matches!(work_status, WorkStatus::Working);
    let seconds_until_end =
        (workday && second < settings.end_minute * 60).then(|| settings.end_minute * 60 - second);
    Ok(Snapshot {
        cycle_start: start.to_string(),
        next_payday: end.to_string(),
        cycle_won: (elapsed as f64 * rate).min(settings.monthly_won as f64),
        today_won: worked_seconds(settings, date, now) as f64 * rate,
        won_per_second: rate,
        working,
        work_status,
        seconds_until_end,
        workdays,
    })
}

pub async fn load(pool: &SqlitePool) -> Result<Option<Settings>, String> {
    let value: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key = 'salary.settings'")
            .fetch_optional(pool)
            .await
            .map_err(|error| error.to_string())?;
    value
        .map(|json| {
            let settings: Settings =
                serde_json::from_str(&json).map_err(|error| error.to_string())?;
            settings.validate()?;
            Ok(settings)
        })
        .transpose()
}

pub async fn save(pool: &SqlitePool, settings: Option<Settings>) -> Result<(), String> {
    if let Some(settings) = settings {
        settings.validate()?;
        let json = serde_json::to_string(&settings).map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO app_settings (key, value) VALUES ('salary.settings', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
            .bind(json).execute(pool).await.map_err(|error| error.to_string())?;
    } else {
        sqlx::query("DELETE FROM app_settings WHERE key = 'salary.settings'")
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            monthly_won: 3_000_000,
            payday: 25,
            start_minute: 540,
            end_minute: 1080,
            lunch_start_minute: 720,
            lunch_end_minute: 780,
            daily_minutes: 480,
            weekdays: vec![0, 1, 2, 3, 4],
        }
    }
    fn at(time: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(time, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn reports_shift_states_and_wall_clock_time_until_end_at_boundaries() {
        let s = settings();
        let before = calculate(&s, at("2026-11-17 08:59:59")).unwrap();
        assert!(matches!(before.work_status, WorkStatus::BeforeWork));
        assert_eq!(before.seconds_until_end, Some(9 * 3600 + 1));
        let started = calculate(&s, at("2026-11-17 09:00:00")).unwrap();
        assert!(matches!(started.work_status, WorkStatus::Working));
        assert!(started.working);
        let lunch = calculate(&s, at("2026-11-17 12:00:00")).unwrap();
        assert!(matches!(lunch.work_status, WorkStatus::Lunch));
        assert!(!lunch.working);
        assert_eq!(lunch.seconds_until_end, Some(6 * 3600));
        let resumed = calculate(&s, at("2026-11-17 13:00:00")).unwrap();
        assert!(matches!(resumed.work_status, WorkStatus::Working));
        let last = calculate(&s, at("2026-11-17 17:59:59")).unwrap();
        assert_eq!(last.seconds_until_end, Some(1));
        let finished = calculate(&s, at("2026-11-17 18:00:00")).unwrap();
        assert!(matches!(finished.work_status, WorkStatus::Finished));
        assert_eq!(finished.seconds_until_end, None);
        let off = calculate(&s, at("2026-11-21 12:30:00")).unwrap();
        assert!(matches!(off.work_status, WorkStatus::DayOff));
        assert_eq!(off.seconds_until_end, None);
        let mut no_lunch = s;
        no_lunch.lunch_end_minute = no_lunch.lunch_start_minute;
        no_lunch.daily_minutes = no_lunch.end_minute - no_lunch.start_minute;
        assert!(matches!(
            calculate(&no_lunch, at("2026-11-17 12:00:00"))
                .unwrap()
                .work_status,
            WorkStatus::Working
        ));
    }

    #[test]
    fn pauses_at_lunch_after_work_and_weekends() {
        let s = settings();
        let noon = calculate(&s, at("2026-11-17 12:00:00")).unwrap();
        let lunch = calculate(&s, at("2026-11-17 12:59:59")).unwrap();
        assert_eq!(noon.cycle_won, lunch.cycle_won);
        assert!(!lunch.working);
        let after = calculate(&s, at("2026-11-17 18:00:00")).unwrap();
        assert_eq!(
            after.cycle_won,
            calculate(&s, at("2026-11-17 23:00:00")).unwrap().cycle_won
        );
        let saturday = calculate(&s, at("2026-11-21 15:00:00")).unwrap();
        assert_eq!(saturday.today_won, 0.0);
        assert!(!saturday.working);
        let tick = calculate(&s, at("2026-11-17 11:00:01")).unwrap();
        let prev = calculate(&s, at("2026-11-17 11:00:00")).unwrap();
        assert!((tick.cycle_won - prev.cycle_won - tick.won_per_second).abs() < 1e-6);
        assert!(tick.working);
    }

    #[test]
    fn resets_on_payday_and_caps_full_cycle() {
        let s = settings();
        let before = calculate(&s, at("2026-11-24 23:59:59")).unwrap();
        assert!((before.cycle_won - s.monthly_won as f64).abs() < 1e-6);
        let reset = calculate(&s, at("2026-11-25 00:00:00")).unwrap();
        assert_eq!(reset.cycle_won, 0.0);
        assert_eq!(reset.cycle_start, "2026-11-25");
        assert_eq!(reset.next_payday, "2026-12-25");
    }

    #[test]
    fn clamps_short_months_and_handles_year_boundary() {
        let mut s = settings();
        s.payday = 31;
        let leap = calculate(&s, at("2028-02-29 00:00:00")).unwrap();
        assert_eq!(leap.cycle_start, "2028-02-29");
        assert_eq!(leap.next_payday, "2028-03-31");
        let feb = calculate(&s, at("2027-02-27 00:00:00")).unwrap();
        assert_eq!(feb.next_payday, "2027-02-28");
        let year = calculate(&s, at("2026-12-31 00:00:00")).unwrap();
        assert_eq!(year.next_payday, "2027-01-31");
    }

    #[test]
    fn rejects_invalid_settings_without_panicking() {
        for change in 0..6 {
            let mut s = settings();
            match change {
                0 => s.monthly_won = 0,
                1 => s.payday = 0,
                2 => s.end_minute = 500,
                3 => s.daily_minutes = 0,
                4 => s.weekdays.clear(),
                _ => s.lunch_start_minute = 1200,
            }
            assert!(calculate(&s, at("2026-11-17 12:00:00")).is_err());
        }
    }

    #[tokio::test]
    async fn persists_validates_and_deletes_settings() {
        let pool = crate::database::connect("sqlite::memory:").await.unwrap();
        assert!(load(&pool).await.unwrap().is_none());
        save(&pool, Some(settings())).await.unwrap();
        assert_eq!(load(&pool).await.unwrap().unwrap().monthly_won, 3_000_000);
        let mut invalid = settings();
        invalid.payday = 32;
        assert!(save(&pool, Some(invalid)).await.is_err());
        assert_eq!(load(&pool).await.unwrap().unwrap().payday, 25);
        save(&pool, None).await.unwrap();
        assert!(load(&pool).await.unwrap().is_none());
    }
}
