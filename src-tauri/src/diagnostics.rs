use chrono::Utc;
use serde_json::{Map, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

const MAX_LOG_BYTES: u64 = 1024 * 1024;
const RETAINED_LOG_FILES: usize = 3;
const LOG_FILE_NAME: &str = "rundev-diagnostics.jsonl";

struct DiagnosticLog {
    path: PathBuf,
    file: Option<File>,
}

static LOG: OnceLock<Mutex<DiagnosticLog>> = OnceLock::new();
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init(app_data_dir: &Path) -> std::io::Result<()> {
    let directory = app_data_dir.join("diagnostics");
    fs::create_dir_all(&directory)?;
    let path = directory.join(LOG_FILE_NAME);
    rotate_if_needed(&path)?;
    let file = open_log(&path)?;
    let _ = LOG_DIR.set(directory);
    let _ = LOG.set(Mutex::new(DiagnosticLog {
        path,
        file: Some(file),
    }));
    record(
        "app_started",
        &[
            ("version", env!("CARGO_PKG_VERSION").to_string()),
            ("os", std::env::consts::OS.to_string()),
            ("arch", std::env::consts::ARCH.to_string()),
        ],
    );
    Ok(())
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
    if serde_json::to_writer(&mut *file, &entry).is_ok() {
        let _ = file.write_all(b"\n");
        let _ = file.flush();
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

fn open_log(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
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
    use super::{rotate, rotated_path, LOG_FILE_NAME};
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
}
