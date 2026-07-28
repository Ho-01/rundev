use std::{path::Path, time::Duration};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::{
            SystemInformation::GetTickCount,
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::{
            Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
        },
    },
};

use super::PlatformSnapshot;

pub(super) fn snapshot() -> PlatformSnapshot {
    let app_identifier = foreground_executable();
    let locked = app_identifier
        .as_deref()
        .is_some_and(|name| matches!(name, "lockapp.exe" | "logonui.exe"));

    PlatformSnapshot {
        app_identifier,
        idle_for: input_idle_time(),
        locked,
    }
}

fn foreground_executable() -> Option<String> {
    unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return None;
        }

        let mut process_id = 0;
        GetWindowThreadProcessId(window, Some(&mut process_id));
        if process_id == 0 {
            return None;
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let mut buffer = vec![0_u16; 32_768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        result.ok()?;

        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
    }
}

fn input_idle_time() -> Duration {
    unsafe {
        let mut info = LASTINPUTINFO {
            cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        if GetLastInputInfo(&mut info).as_bool() {
            Duration::from_millis(u64::from(GetTickCount().wrapping_sub(info.dwTime)))
        } else {
            Duration::MAX
        }
    }
}
