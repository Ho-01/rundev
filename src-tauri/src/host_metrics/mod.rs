use serde::Serialize;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc, Mutex, RwLock,
};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Components, Disks, Networks, RefreshKind, System};
use tauri::{AppHandle, Emitter, Manager};

const BACKGROUND_INTERVAL: Duration = Duration::from_secs(10);
const SUMMARY_INTERVAL: Duration = Duration::from_secs(3);
const DETAIL_INTERVAL: Duration = Duration::from_secs(1);
const DISK_INTERVAL: Duration = Duration::from_secs(30);
const BATTERY_INTERVAL: Duration = Duration::from_secs(15);
const TEMP_INTERVAL: Duration = Duration::from_secs(5);
const GAP_RESET: Duration = Duration::from_secs(5);
const EVENT_NAME: &str = "system-stats-updated";
static SAMPLING_MODE: AtomicU8 = AtomicU8::new(0);

pub fn set_sampling_mode(mode: &str) -> Result<(), String> {
    let mode = match mode {
        "background" => 0,
        "summary" => 1,
        "detail" => 2,
        _ => return Err(format!("unsupported host metrics mode: {mode}")),
    };
    SAMPLING_MODE.store(mode, Ordering::Relaxed);
    Ok(())
}

fn sampling_interval(mode: u8) -> Duration {
    match mode {
        2 => DETAIL_INTERVAL,
        1 => SUMMARY_INTERVAL,
        _ => BACKGROUND_INTERVAL,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_percent: Option<f32>,
    pub logical_cpu_count: usize,
    pub memory_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_available_bytes: u64,
    pub temperature_celsius: Option<f32>,
    pub temperature_max_celsius: Option<f32>,
    pub battery_percent: Option<f32>,
    pub battery_state: Option<String>,
    pub disk_percent: Option<f32>,
    pub disk_total_bytes: Option<u64>,
    pub disk_used_bytes: Option<u64>,
    pub disk_available_bytes: Option<u64>,
    pub network_down_bps: Option<f64>,
    pub network_up_bps: Option<f64>,
    pub sequence: u64,
}

#[derive(Clone)]
struct BatterySnapshot {
    percent: Option<f32>,
    state: Option<String>,
}

struct HostMetricsState {
    latest: Arc<RwLock<SystemStats>>,
}

impl HostMetricsState {
    fn snapshot(&self) -> SystemStats {
        self.latest
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
}

pub fn start(app: AppHandle) {
    let latest = Arc::new(RwLock::new(SystemStats {
        cpu_percent: None,
        logical_cpu_count: 0,
        memory_percent: 0.0,
        memory_total_bytes: 0,
        memory_used_bytes: 0,
        memory_available_bytes: 0,
        temperature_celsius: None,
        temperature_max_celsius: None,
        battery_percent: None,
        battery_state: None,
        disk_percent: None,
        disk_total_bytes: None,
        disk_used_bytes: None,
        disk_available_bytes: None,
        network_down_bps: None,
        network_up_bps: None,
        sequence: 0,
    }));
    let battery = Arc::new(Mutex::new(BatterySnapshot {
        percent: None,
        state: None,
    }));

    start_battery_thread(Arc::clone(&battery));

    app.manage(HostMetricsState {
        latest: Arc::clone(&latest),
    });

    let emit_latest = Arc::clone(&latest);
    let emit_battery = Arc::clone(&battery);
    thread::Builder::new()
        .name("rundev-host-metrics".into())
        .spawn(move || {
            run_sampler(app, emit_latest, emit_battery);
        })
        .expect("failed to start host metrics sampler");
}

pub fn current_stats(app: &AppHandle) -> SystemStats {
    match app.try_state::<HostMetricsState>() {
        Some(state) => state.snapshot(),
        None => SystemStats {
            cpu_percent: None,
            logical_cpu_count: 0,
            memory_percent: 0.0,
            memory_total_bytes: 0,
            memory_used_bytes: 0,
            memory_available_bytes: 0,
            temperature_celsius: None,
            temperature_max_celsius: None,
            battery_percent: None,
            battery_state: None,
            disk_percent: None,
            disk_total_bytes: None,
            disk_used_bytes: None,
            disk_available_bytes: None,
            network_down_bps: None,
            network_up_bps: None,
            sequence: 0,
        },
    }
}

fn start_battery_thread(shared: Arc<Mutex<BatterySnapshot>>) {
    thread::Builder::new()
        .name("rundev-battery".into())
        .spawn(move || {
            let Ok(manager) = battery::Manager::new() else {
                return;
            };
            loop {
                let snapshot = sample_battery(&manager);
                if let Ok(mut guard) = shared.lock() {
                    *guard = snapshot;
                }
                thread::sleep(BATTERY_INTERVAL);
            }
        })
        .ok();
}

fn sample_battery(manager: &battery::Manager) -> BatterySnapshot {
    let Ok(batteries) = manager.batteries() else {
        return BatterySnapshot {
            percent: None,
            state: None,
        };
    };

    let mut energy = 0.0_f64;
    let mut full = 0.0_f64;
    let mut state = None::<String>;
    let mut found = false;

    for battery in batteries.flatten() {
        found = true;
        energy += battery.energy().value as f64;
        full += battery.energy_full().value as f64;
        if state.is_none() {
            state = Some(match battery.state() {
                battery::State::Charging => "charging".into(),
                battery::State::Discharging => "discharging".into(),
                battery::State::Full => "full".into(),
                battery::State::Empty => "discharging".into(),
                _ => "unknown".into(),
            });
        }
    }

    if !found {
        return BatterySnapshot {
            percent: None,
            state: None,
        };
    }

    let percent = if full > 0.0 {
        Some(((energy / full) * 100.0).clamp(0.0, 100.0) as f32)
    } else {
        None
    };

    BatterySnapshot { percent, state }
}

fn run_sampler(
    app: AppHandle,
    latest: Arc<RwLock<SystemStats>>,
    battery: Arc<Mutex<BatterySnapshot>>,
) {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(sysinfo::CpuRefreshKind::everything())
            .with_memory(sysinfo::MemoryRefreshKind::everything()),
    );
    let mut networks = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list();
    let mut components = Components::new_with_refreshed_list();

    let mut last_tick = Instant::now();
    let mut last_disk = Instant::now()
        .checked_sub(DISK_INTERVAL)
        .unwrap_or_else(Instant::now);
    let mut last_temp = Instant::now()
        .checked_sub(TEMP_INTERVAL)
        .unwrap_or_else(Instant::now);
    let mut network_ready = false;
    let mut sequence = 0_u64;
    let mut disk = read_disk_snapshot(&disks);
    let mut temperature_celsius = read_temperature_celsius(&components);
    let mut temperature_max_celsius = temperature_celsius;
    let mut cpu_warmed = false;
    let mut last_mode = SAMPLING_MODE.load(Ordering::Relaxed);
    let mut next_sample = Instant::now();

    // Prime CPU counters without publishing a misleading 0%.
    system.refresh_cpu_usage();
    thread::sleep(Duration::from_millis(200));
    system.refresh_cpu_usage();
    system.refresh_memory();

    loop {
        let now = Instant::now();
        let mode = SAMPLING_MODE.load(Ordering::Relaxed);
        if mode != last_mode {
            last_mode = mode;
            next_sample = now;
        }
        if now < next_sample {
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        let elapsed = now.saturating_duration_since(last_tick);
        last_tick = now;

        let gap_reset = elapsed > GAP_RESET;
        if gap_reset {
            cpu_warmed = false;
            network_ready = false;
            system.refresh_cpu_usage();
            networks.refresh(true);
        }

        system.refresh_cpu_usage();
        system.refresh_memory();
        networks.refresh(true);

        if now.duration_since(last_disk) >= DISK_INTERVAL {
            disks.refresh(true);
            disk = read_disk_snapshot(&disks);
            last_disk = now;
        }

        if now.duration_since(last_temp) >= TEMP_INTERVAL {
            components.refresh(true);
            temperature_celsius = read_temperature_celsius(&components);
            if let Some(temperature) = temperature_celsius {
                temperature_max_celsius = Some(
                    temperature_max_celsius
                        .map(|current| current.max(temperature))
                        .unwrap_or(temperature),
                );
            }
            last_temp = now;
        }

        let cpu_percent = if cpu_warmed && !gap_reset {
            Some(system.global_cpu_usage().clamp(0.0, 100.0))
        } else {
            cpu_warmed = true;
            None
        };

        let memory_total_bytes = system.total_memory();
        let memory_used_bytes = system.used_memory();
        let memory_available_bytes = system.available_memory();
        let total_memory = memory_total_bytes.max(1) as f64;
        let memory_percent =
            ((memory_used_bytes as f64 / total_memory) * 100.0).clamp(0.0, 100.0) as f32;

        let (network_down_bps, network_up_bps) = if gap_reset || !network_ready {
            network_ready = true;
            (None, None)
        } else {
            let secs = elapsed.as_secs_f64().max(0.001);
            let mut down = 0_u64;
            let mut up = 0_u64;
            for (_name, data) in networks.iter() {
                down = down.saturating_add(data.received());
                up = up.saturating_add(data.transmitted());
            }
            (Some(down as f64 / secs), Some(up as f64 / secs))
        };

        let battery_snap = battery
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or(BatterySnapshot {
                percent: None,
                state: None,
            });

        sequence = sequence.wrapping_add(1);
        let stats = SystemStats {
            cpu_percent,
            logical_cpu_count: system.cpus().len(),
            memory_percent,
            memory_total_bytes,
            memory_used_bytes,
            memory_available_bytes,
            temperature_celsius,
            temperature_max_celsius,
            battery_percent: battery_snap.percent,
            battery_state: battery_snap.state,
            disk_percent: disk.as_ref().map(|snapshot| snapshot.percent),
            disk_total_bytes: disk.as_ref().map(|snapshot| snapshot.total_bytes),
            disk_used_bytes: disk.as_ref().map(|snapshot| snapshot.used_bytes),
            disk_available_bytes: disk.as_ref().map(|snapshot| snapshot.available_bytes),
            network_down_bps,
            network_up_bps,
            sequence,
        };

        if let Ok(mut guard) = latest.write() {
            *guard = stats.clone();
        }

        if let Err(error) = app.emit(EVENT_NAME, &stats) {
            tracing::debug!(%error, "system stats emit skipped");
        }

        next_sample = Instant::now() + sampling_interval(mode);
    }
}

fn read_temperature_celsius(components: &Components) -> Option<f32> {
    let mut cpu_temp = None::<f32>;
    let mut any_temp = None::<f32>;

    for component in components.list() {
        let Some(temp) = component.temperature() else {
            continue;
        };
        if !temp.is_finite() || temp <= 0.0 || temp > 125.0 {
            continue;
        }
        let label = component.label().to_ascii_lowercase();
        if label.contains("cpu")
            || label.contains("package")
            || label.contains("tdie")
            || label.contains("core")
            || label.contains("soc")
        {
            cpu_temp = Some(match cpu_temp {
                Some(current) => current.max(temp),
                None => temp,
            });
        } else {
            any_temp = Some(match any_temp {
                Some(current) => current.max(temp),
                None => temp,
            });
        }
    }

    cpu_temp.or(any_temp)
}

#[derive(Clone)]
struct DiskSnapshot {
    percent: f32,
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
}

fn read_disk_snapshot(disks: &Disks) -> Option<DiskSnapshot> {
    let target = primary_mount_hint();
    let mut fallback = None;

    for disk in disks.list() {
        let mount = disk.mount_point();
        let total = disk.total_space();
        if total == 0 {
            continue;
        }
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let percent = ((used as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32;
        let snapshot = DiskSnapshot {
            percent,
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
        };

        if mount_matches(mount, &target) {
            return Some(snapshot);
        }
        if fallback.is_none() {
            fallback = Some(snapshot);
        }
    }

    fallback
}

fn primary_mount_hint() -> String {
    #[cfg(windows)]
    {
        std::env::var("SystemDrive")
            .ok()
            .map(|drive| {
                if drive.ends_with(':') {
                    format!("{drive}\\")
                } else {
                    format!("{drive}:\\")
                }
            })
            .unwrap_or_else(|| "C:\\".into())
    }
    #[cfg(not(windows))]
    {
        "/".into()
    }
}

fn mount_matches(mount: &Path, target: &str) -> bool {
    let mount_str = mount.to_string_lossy();
    #[cfg(windows)]
    {
        let normalized_mount = mount_str.trim_end_matches(['\\', '/']).to_ascii_lowercase();
        let normalized_target = target.trim_end_matches(['\\', '/']).to_ascii_lowercase();
        normalized_mount == normalized_target
            || normalized_mount.starts_with(&normalized_target)
            || normalized_target.starts_with(&normalized_mount)
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        mount_str == "/"
    }
}

#[cfg(test)]
mod tests {
    use super::{mount_matches, sampling_interval};
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn sampling_modes_have_distinct_cost_profiles() {
        assert_eq!(sampling_interval(0), Duration::from_secs(10));
        assert_eq!(sampling_interval(1), Duration::from_secs(3));
        assert_eq!(sampling_interval(2), Duration::from_secs(1));
    }

    #[test]
    fn matches_unix_root() {
        #[cfg(not(windows))]
        assert!(mount_matches(Path::new("/"), "/"));
    }

    #[test]
    fn matches_windows_system_drive() {
        #[cfg(windows)]
        {
            assert!(mount_matches(Path::new("C:\\"), "C:\\"));
            assert!(mount_matches(Path::new("c:\\"), "C:"));
        }
    }
}
