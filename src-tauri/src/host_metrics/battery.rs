use super::BatterySnapshot;

#[cfg(windows)]
pub(super) fn sample() -> BatterySnapshot {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

    let mut status = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut status) }.is_err()
        || status.BatteryFlag == 128
        || status.BatteryLifePercent == u8::MAX
    {
        return unavailable();
    }

    let state = if status.BatteryFlag & 8 != 0 {
        "charging"
    } else if status.BatteryFlag & 4 != 0 || status.BatteryLifePercent >= 100 {
        "full"
    } else if status.ACLineStatus == 0 {
        "discharging"
    } else {
        "unknown"
    };

    BatterySnapshot {
        percent: Some(f32::from(status.BatteryLifePercent).clamp(0.0, 100.0)),
        state: Some(state.to_string()),
    }
}

#[cfg(target_os = "macos")]
pub(super) fn sample() -> BatterySnapshot {
    let output = std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            parse_pmset(&String::from_utf8_lossy(&output.stdout)).unwrap_or_else(unavailable)
        }
        _ => unavailable(),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub(super) fn sample() -> BatterySnapshot {
    unavailable()
}

fn unavailable() -> BatterySnapshot {
    BatterySnapshot {
        percent: None,
        state: None,
    }
}

#[cfg(any(test, target_os = "macos"))]
fn parse_pmset(output: &str) -> Option<BatterySnapshot> {
    let line = output.lines().find(|line| line.contains('%'))?;
    let percent_end = line.find('%')?;
    let percent_start = line[..percent_end]
        .rfind(|character: char| !character.is_ascii_digit())
        .map_or(0, |index| index + 1);
    let percent = line[percent_start..percent_end].parse::<f32>().ok()?;
    let normalized = line.to_ascii_lowercase();
    let state = if normalized.contains("discharging") {
        "discharging"
    } else if normalized.contains("not charging") {
        if percent >= 100.0 {
            "full"
        } else {
            "unknown"
        }
    } else if normalized.contains("charging") || normalized.contains("finishing charge") {
        "charging"
    } else if normalized.contains("charged") {
        "full"
    } else {
        "unknown"
    };

    Some(BatterySnapshot {
        percent: Some(percent.clamp(0.0, 100.0)),
        state: Some(state.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_pmset;

    #[test]
    fn parses_charging_pmset_output() {
        let output = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t87%; charging; 0:42 remaining present: true";
        let snapshot = parse_pmset(output).unwrap();

        assert_eq!(snapshot.percent, Some(87.0));
        assert_eq!(snapshot.state.as_deref(), Some("charging"));
    }

    #[test]
    fn parses_discharging_pmset_output() {
        let output = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t54%; discharging; 4:20 remaining present: true";
        let snapshot = parse_pmset(output).unwrap();

        assert_eq!(snapshot.percent, Some(54.0));
        assert_eq!(snapshot.state.as_deref(), Some("discharging"));
    }

    #[test]
    fn ignores_output_without_a_battery_percentage() {
        assert!(parse_pmset("Now drawing from 'AC Power'").is_none());
    }

    #[test]
    fn does_not_treat_not_charging_as_charging() {
        let output = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t80%; not charging; present: true";
        let snapshot = parse_pmset(output).unwrap();

        assert_eq!(snapshot.state.as_deref(), Some("unknown"));
    }
}
