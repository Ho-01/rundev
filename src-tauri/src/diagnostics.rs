use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
        Mutex, OnceLock,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 1024 * 1024;
const RETAINED_LOG_FILES: usize = 3;
const LOG_FILE_NAME: &str = "rundev-diagnostics.jsonl";
const RUNTIME_STATE_FILE_NAME: &str = "runtime-state.json";
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const RETAINED_CRASH_DUMPS: usize = 2;

const COMPONENT_APP: u8 = 0;
const COMPONENT_ACTIVITY: u8 = 1;
const COMPONENT_KEYBOARD: u8 = 2;
const COMPONENT_HOST_METRICS: u8 = 3;
const COMPONENT_BATTERY: u8 = 4;
const COMPONENT_TRAY: u8 = 5;

struct DiagnosticLog {
    path: PathBuf,
    file: Option<File>,
}

static LOG: OnceLock<Mutex<DiagnosticLog>> = OnceLock::new();
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();
static SESSION_ID: OnceLock<String> = OnceLock::new();
static SESSION_STARTED_MS: AtomicU64 = AtomicU64::new(0);
static LAST_HEARTBEAT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_COMPONENT: AtomicU8 = AtomicU8::new(COMPONENT_APP);
static COMPONENT_HEARTBEATS: [AtomicU64; 6] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static RUNTIME_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static CRASH_DUMP_FILE: OnceLock<File> = OnceLock::new();
#[cfg(windows)]
static CRASH_DUMP_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeState {
    status: String,
    session_id: String,
    version: String,
    os: String,
    process_id: u32,
    #[serde(default)]
    debug_build: bool,
    started_at_ms: u64,
    heartbeat_at_ms: u64,
    last_component: String,
    component_heartbeats_ms: BTreeMap<String, u64>,
}

pub fn init(app_data_dir: &Path) -> std::io::Result<()> {
    let directory = app_data_dir.join("diagnostics");
    fs::create_dir_all(&directory)?;
    let path = directory.join(LOG_FILE_NAME);
    rotate_if_needed(&path)?;
    write_diagnostics_readme(&directory)?;
    let file = open_log(&path)?;
    let _ = LOG_DIR.set(directory.clone());
    let _ = LOG.set(Mutex::new(DiagnosticLog {
        path,
        file: Some(file),
    }));
    inspect_previous_runtime_state(&directory);

    let now = unix_time_ms();
    let session_id = uuid::Uuid::new_v4().to_string();
    let _ = SESSION_ID.set(session_id);
    SESSION_STARTED_MS.store(now, Ordering::Relaxed);
    LAST_HEARTBEAT_MS.store(now, Ordering::Relaxed);
    for heartbeat in &COMPONENT_HEARTBEATS {
        heartbeat.store(now, Ordering::Relaxed);
    }
    RUNTIME_ACTIVE.store(true, Ordering::Release);

    #[cfg(windows)]
    install_windows_crash_dump(&directory)?;

    install_panic_diagnostics();
    persist_runtime_state("running");
    start_runtime_heartbeat();
    record(
        "app_started",
        &[
            ("version", env!("CARGO_PKG_VERSION").to_string()),
            ("os", std::env::consts::OS.to_string()),
            ("arch", std::env::consts::ARCH.to_string()),
            ("process_id", std::process::id().to_string()),
        ],
    );
    Ok(())
}

pub fn heartbeat(component: &'static str) {
    let component = component_index(component);
    let now = unix_time_ms();
    COMPONENT_HEARTBEATS[usize::from(component)].store(now, Ordering::Relaxed);
    LAST_COMPONENT.store(component, Ordering::Relaxed);
}

pub fn mark_clean_exit() {
    if RUNTIME_ACTIVE.swap(false, Ordering::AcqRel) {
        heartbeat("app");
        LAST_HEARTBEAT_MS.store(unix_time_ms(), Ordering::Relaxed);
        persist_runtime_state("clean");
        record("app_exit_clean", &[]);
    }
}

pub fn record(event: &str, fields: &[(&str, String)]) {
    let Some(log) = LOG.get() else {
        return;
    };
    let Ok(mut log) = log.lock() else {
        return;
    };

    if log
        .file
        .as_ref()
        .and_then(|file| file.metadata().ok())
        .is_some_and(|metadata| metadata.len() >= MAX_LOG_BYTES)
    {
        log.file.take();
        if rotate(&log.path).is_ok() {
            log.file = open_log(&log.path).ok();
        }
    }

    let Some(file) = log.file.as_mut() else {
        return;
    };
    let mut safe_fields = Map::new();
    for (key, value) in fields {
        safe_fields.insert((*key).to_string(), Value::String(value.clone()));
    }
    let entry = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "event": event,
        "fields": safe_fields,
    });
    if let Ok(mut encoded) = serde_json::to_vec(&entry) {
        encoded.push(b'\n');
        if file.write_all(&encoded).is_ok() {
            let _ = file.flush();
        }
    }
}

pub fn open_folder() -> Result<(), String> {
    let directory = LOG_DIR
        .get()
        .ok_or_else(|| "진단 로그 폴더가 아직 준비되지 않았습니다.".to_string())?;

    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(windows)]
    let mut command = Command::new("explorer");
    #[cfg(not(any(target_os = "macos", windows)))]
    let mut command = Command::new("xdg-open");

    command
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("진단 로그 폴더를 열지 못했습니다: {error}"))
}

fn component_index(component: &str) -> u8 {
    match component {
        "activity" => COMPONENT_ACTIVITY,
        "keyboard" => COMPONENT_KEYBOARD,
        "host_metrics" => COMPONENT_HOST_METRICS,
        "battery" => COMPONENT_BATTERY,
        "tray" => COMPONENT_TRAY,
        _ => COMPONENT_APP,
    }
}

fn component_name(component: u8) -> &'static str {
    match component {
        COMPONENT_ACTIVITY => "activity",
        COMPONENT_KEYBOARD => "keyboard",
        COMPONENT_HOST_METRICS => "host_metrics",
        COMPONENT_BATTERY => "battery",
        COMPONENT_TRAY => "tray",
        _ => "app",
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn runtime_state_path(directory: &Path) -> PathBuf {
    directory.join(RUNTIME_STATE_FILE_NAME)
}

fn inspect_previous_runtime_state(directory: &Path) {
    let path = runtime_state_path(directory);
    let Ok(encoded) = fs::read(&path) else {
        return;
    };
    match serde_json::from_slice::<RuntimeState>(&encoded) {
        Ok(previous) if previous.status == "running" => {
            let heartbeat_age_ms = unix_time_ms().saturating_sub(previous.heartbeat_at_ms);
            let event = previous_run_event(previous.debug_build, heartbeat_age_ms);
            if event == "previous_run_unclean" {
                let _ = fs::write(
                    directory.join("previous-runtime-state.json"),
                    serde_json::to_vec_pretty(&previous).unwrap_or_default(),
                );
            }
            let mut fields = vec![
                ("previous_version", previous.version),
                ("previous_os", previous.os),
                ("previous_process_id", previous.process_id.to_string()),
                ("previous_started_at_ms", previous.started_at_ms.to_string()),
                (
                    "previous_heartbeat_at_ms",
                    previous.heartbeat_at_ms.to_string(),
                ),
                ("heartbeat_age_ms", heartbeat_age_ms.to_string()),
                ("previous_last_component", previous.last_component),
            ];
            for component in [
                "app",
                "activity",
                "keyboard",
                "host_metrics",
                "battery",
                "tray",
            ] {
                if let Some(timestamp) = previous.component_heartbeats_ms.get(component) {
                    let field_name = match component {
                        "app" => "previous_app_heartbeat_ms",
                        "activity" => "previous_activity_heartbeat_ms",
                        "keyboard" => "previous_keyboard_heartbeat_ms",
                        "host_metrics" => "previous_host_metrics_heartbeat_ms",
                        "battery" => "previous_battery_heartbeat_ms",
                        _ => "previous_tray_heartbeat_ms",
                    };
                    fields.push((field_name, timestamp.to_string()));
                }
            }
            record(event, &fields);
        }
        Err(error) => record(
            "previous_runtime_state_unreadable",
            &[("error_kind", format!("{:?}", error.classify()))],
        ),
        _ => {}
    }
}

fn previous_run_event(debug_build: bool, heartbeat_age_ms: u64) -> &'static str {
    if debug_build && heartbeat_age_ms < 10 * 60 * 1_000 {
        "previous_dev_run_interrupted"
    } else {
        "previous_run_unclean"
    }
}

fn current_runtime_state(status: &str) -> Option<RuntimeState> {
    let session_id = SESSION_ID.get()?.clone();
    let mut component_heartbeats_ms = BTreeMap::new();
    for component in 0..COMPONENT_HEARTBEATS.len() {
        component_heartbeats_ms.insert(
            component_name(component as u8).to_string(),
            COMPONENT_HEARTBEATS[component].load(Ordering::Relaxed),
        );
    }
    Some(RuntimeState {
        status: status.to_string(),
        session_id,
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        process_id: std::process::id(),
        debug_build: cfg!(debug_assertions),
        started_at_ms: SESSION_STARTED_MS.load(Ordering::Relaxed),
        heartbeat_at_ms: LAST_HEARTBEAT_MS.load(Ordering::Relaxed),
        last_component: component_name(LAST_COMPONENT.load(Ordering::Relaxed)).to_string(),
        component_heartbeats_ms,
    })
}

fn persist_runtime_state(status: &str) {
    let (Some(directory), Some(state)) = (LOG_DIR.get(), current_runtime_state(status)) else {
        return;
    };
    let Ok(encoded) = serde_json::to_vec_pretty(&state) else {
        return;
    };
    let path = runtime_state_path(directory);
    if let Ok(mut file) = File::create(path) {
        let _ = file.write_all(&encoded);
        let _ = file.flush();
    }
}

fn start_runtime_heartbeat() {
    thread::Builder::new()
        .name("rundev-diagnostics-heartbeat".into())
        .spawn(|| {
            while RUNTIME_ACTIVE.load(Ordering::Acquire) {
                thread::sleep(HEARTBEAT_INTERVAL);
                if !RUNTIME_ACTIVE.load(Ordering::Acquire) {
                    break;
                }
                LAST_HEARTBEAT_MS.store(unix_time_ms(), Ordering::Relaxed);
                persist_runtime_state("running");
            }
        })
        .ok();
}

fn install_panic_diagnostics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread_name = thread::current().name().unwrap_or("unnamed").to_string();
        let mut fields = vec![("thread", thread_name)];
        if let Some(location) = info.location() {
            fields.push(("line", location.line().to_string()));
            fields.push(("column", location.column().to_string()));
        }
        record("rust_panic", &fields);
        persist_runtime_state("panicking");
        previous(info);
    }));
}

#[cfg(windows)]
fn install_windows_crash_dump(directory: &Path) -> std::io::Result<()> {
    use windows::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter;

    rotate_crash_dumps(directory)?;
    let pending = directory.join("pending-crash-filtered.dmp");
    let file = File::create(pending)?;
    let _ = CRASH_DUMP_FILE.set(file);
    unsafe {
        SetUnhandledExceptionFilter(Some(windows_unhandled_exception));
    }
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn windows_unhandled_exception(
    exception: *const windows::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
) -> i32 {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::FlushFileBuffers,
        System::{
            Diagnostics::Debug::{
                MiniDumpFilterMemory, MiniDumpFilterModulePaths, MiniDumpWithThreadInfo,
                MiniDumpWithUnloadedModules, MiniDumpWithoutOptionalData, MiniDumpWriteDump,
                MINIDUMP_EXCEPTION_INFORMATION,
            },
            Threading::{GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId},
        },
    };

    if CRASH_DUMP_STARTED.swap(true, Ordering::AcqRel) {
        return windows::Win32::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    }
    let Some(file) = CRASH_DUMP_FILE.get() else {
        return windows::Win32::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH;
    };
    let info = MINIDUMP_EXCEPTION_INFORMATION {
        ThreadId: unsafe { GetCurrentThreadId() },
        ExceptionPointers: exception.cast_mut(),
        ClientPointers: false.into(),
    };
    let dump_type = MiniDumpFilterMemory
        | MiniDumpFilterModulePaths
        | MiniDumpWithoutOptionalData
        | MiniDumpWithThreadInfo
        | MiniDumpWithUnloadedModules;
    let handle = HANDLE(file.as_raw_handle());
    let dumped = unsafe {
        MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            handle,
            dump_type,
            Some(&info),
            None,
            None,
        )
    };
    if dumped.is_ok() {
        let _ = unsafe { FlushFileBuffers(handle) };
    }
    windows::Win32::System::Diagnostics::Debug::EXCEPTION_CONTINUE_SEARCH
}

#[cfg(windows)]
fn rotate_crash_dumps(directory: &Path) -> std::io::Result<()> {
    let pending = directory.join("pending-crash-filtered.dmp");
    if pending.metadata().is_ok_and(|metadata| metadata.len() > 0) {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        fs::rename(
            &pending,
            directory.join(format!("crash-{timestamp}-filtered.dmp")),
        )?;
    } else if pending.exists() {
        fs::remove_file(&pending)?;
    }

    let mut dumps = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("crash-") && name.ends_with("-filtered.dmp"))
        })
        .collect::<Vec<_>>();
    dumps.sort();
    let remove_count = dumps.len().saturating_sub(RETAINED_CRASH_DUMPS);
    for path in dumps.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn open_log(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

fn write_diagnostics_readme(directory: &Path) -> std::io::Result<()> {
    let guidance = "RunDev 로컬 진단 파일\r\n\r\n\
- rundev-diagnostics.jsonl: 허용된 상태 이벤트와 비정상 종료 요약\r\n\
- runtime-state.json: 현재 실행 heartbeat\r\n\
- previous-runtime-state.json: 마지막 비정상 종료 직전 heartbeat\r\n\
- crash-*-filtered.dmp: Windows 필터 스택 덤프(생성된 경우, 최근 2개)\r\n\r\n\
문제 신고 시 위 파일을 함께 전달하면 마지막 정상 서브시스템과 네이티브 스택을 대조할 수 있습니다.\r\n\
파일은 자동 전송되지 않습니다. 필터 덤프는 전체 메모리를 포함하지 않지만 진단 파일이므로 공개 게시하지 마세요.\r\n\
macOS에서는 ~/Library/Logs/DiagnosticReports 안의 RunDev 관련 .ips 파일도 함께 전달하세요.\r\n";
    fs::write(directory.join("README.txt"), guidance)
}

fn rotate_if_needed(path: &Path) -> std::io::Result<()> {
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES)
    {
        rotate(path)?;
    }
    Ok(())
}

fn rotate(path: &Path) -> std::io::Result<()> {
    for index in (1..RETAINED_LOG_FILES).rev() {
        let source = rotated_path(path, index - 1);
        let destination = rotated_path(path, index);
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        if source.exists() {
            fs::rename(source, destination)?;
        }
    }
    Ok(())
}

fn rotated_path(path: &Path, index: usize) -> PathBuf {
    if index == 0 {
        path.to_path_buf()
    } else {
        path.with_file_name(format!("{LOG_FILE_NAME}.{index}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{previous_run_event, rotate, rotated_path, LOG_FILE_NAME};
    use std::fs;

    #[test]
    fn rotates_current_log_and_keeps_three_files() {
        let directory =
            std::env::temp_dir().join(format!("rundev-diagnostics-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let current = directory.join(LOG_FILE_NAME);
        fs::write(&current, "current").unwrap();
        fs::write(rotated_path(&current, 1), "previous").unwrap();
        fs::write(rotated_path(&current, 2), "oldest").unwrap();

        rotate(&current).unwrap();

        assert!(!current.exists());
        assert_eq!(
            fs::read_to_string(rotated_path(&current, 1)).unwrap(),
            "current"
        );
        assert_eq!(
            fs::read_to_string(rotated_path(&current, 2)).unwrap(),
            "previous"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn distinguishes_dev_reload_from_an_old_unclean_run() {
        assert_eq!(
            previous_run_event(true, 30_000),
            "previous_dev_run_interrupted"
        );
        assert_eq!(
            previous_run_event(true, 11 * 60 * 1_000),
            "previous_run_unclean"
        );
        assert_eq!(previous_run_event(false, 30_000), "previous_run_unclean");
    }
}
