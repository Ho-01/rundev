use super::{set_status, KeyEvent, STATUS_ACTIVE, STATUS_ERROR, STATUS_PERMISSION_REQUIRED};
use sqlx::SqlitePool;
use std::{
    ffi::c_void,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        OnceLock,
    },
};
use tokio::sync::mpsc::UnboundedSender;

type CGEventTapProxy = *mut c_void;
type CGEventRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFStringRef = *const c_void;

const CG_SESSION_EVENT_TAP: u32 = 1;
const CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
const CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;
const CG_EVENT_KEY_DOWN: u32 = 10;
const CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = u32::MAX - 1;
const CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = u32::MAX;
const CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

static SENDER: OnceLock<UnboundedSender<KeyEvent>> = OnceLock::new();
static EVENT_TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static FIRST_EVENT_RECORDED: AtomicBool = AtomicBool::new(false);

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: unsafe extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef,
        user_info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFRunLoopCommonModes: CFStringRef;
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: CFMachPortRef,
        order: isize,
    ) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(run_loop: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRun();
}

pub(super) async fn start(sender: UnboundedSender<KeyEvent>, pool: SqlitePool) {
    let initial_permission = has_permission();
    crate::diagnostics::record(
        "keyboard_macos_permission_preflight",
        &[("granted", initial_permission.to_string())],
    );
    if !initial_permission {
        set_status(STATUS_PERMISSION_REQUIRED);
        let prompted: Option<String> = sqlx::query_scalar(
            "SELECT value FROM app_settings WHERE key = 'keyboard.macos.permission_prompted'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        let granted = unsafe { CGRequestListenEventAccess() };
        crate::diagnostics::record(
            "keyboard_macos_permission_requested",
            &[("granted", granted.to_string())],
        );
        if !granted && prompted.as_deref() != Some("true") {
            let _ = open_permission_settings();
            let _ = sqlx::query(
                "INSERT INTO app_settings (key, value)
                 VALUES ('keyboard.macos.permission_prompted', 'true')
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )
            .execute(&pool)
            .await;
        }
    }

    if let Err(error) = std::thread::Builder::new()
        .name("rundev-keyboard-event-tap".to_string())
        .spawn(move || {
            let mut waiting_recorded = false;
            while !has_permission() {
                set_status(STATUS_PERMISSION_REQUIRED);
                if !waiting_recorded {
                    waiting_recorded = true;
                    crate::diagnostics::record("keyboard_macos_waiting_for_permission", &[]);
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            crate::diagnostics::record("keyboard_macos_permission_granted", &[]);
            unsafe {
                if SENDER.set(sender).is_err() {
                    crate::diagnostics::record("keyboard_macos_sender_registration_failed", &[]);
                    set_status(STATUS_ERROR);
                    return;
                }
                let mask = 1_u64 << CG_EVENT_KEY_DOWN;
                let tap = CGEventTapCreate(
                    CG_SESSION_EVENT_TAP,
                    CG_HEAD_INSERT_EVENT_TAP,
                    CG_EVENT_TAP_OPTION_LISTEN_ONLY,
                    mask,
                    keyboard_tap,
                    std::ptr::null_mut(),
                );
                if tap.is_null() {
                    crate::diagnostics::record("keyboard_macos_event_tap_create_failed", &[]);
                    set_status(STATUS_ERROR);
                    return;
                }
                EVENT_TAP.store(tap, Ordering::Relaxed);
                crate::diagnostics::record("keyboard_macos_event_tap_created", &[]);
                let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                if source.is_null() {
                    crate::diagnostics::record("keyboard_macos_run_loop_source_create_failed", &[]);
                    set_status(STATUS_ERROR);
                    return;
                }
                CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                set_status(STATUS_ACTIVE);
                crate::diagnostics::record("keyboard_macos_event_tap_enabled", &[]);
                CFRunLoopRun();
            }
        })
    {
        tracing::error!(%error, "Failed to start macOS keyboard event tap");
        set_status(STATUS_ERROR);
    }
}

pub(super) fn open_permission_settings() -> Result<(), String> {
    let result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string());
    crate::diagnostics::record(
        "keyboard_macos_permission_settings_opened",
        &[("success", result.is_ok().to_string())],
    );
    result
}

pub(super) async fn reset_permission(pool: &SqlitePool) -> Result<(), String> {
    crate::diagnostics::record("keyboard_macos_permission_repair_started", &[]);

    let prompt_reset = sqlx::query(
        "DELETE FROM app_settings
         WHERE key = 'keyboard.macos.permission_prompted'",
    )
    .execute(pool)
    .await;
    crate::diagnostics::record(
        "keyboard_macos_permission_prompt_state_reset",
        &[("success", prompt_reset.is_ok().to_string())],
    );

    let reset_succeeded = Command::new("/usr/bin/tccutil")
        .args(["reset", "ListenEvent", "dev.rundev.app"])
        .status()
        .is_ok_and(|status| status.success());
    crate::diagnostics::record(
        "keyboard_macos_permission_reset",
        &[("success", reset_succeeded.to_string())],
    );

    let _ = open_permission_settings();
    Ok(())
}

pub(super) fn refresh_permission_status() {
    if !has_permission() {
        set_status(STATUS_PERMISSION_REQUIRED);
    }
}

fn has_permission() -> bool {
    unsafe { CGPreflightListenEventAccess() }
}

fn is_modifier_keycode(keycode: u32) -> bool {
    matches!(keycode, 54 | 55 | 56 | 57 | 58 | 59 | 60 | 61 | 62 | 63)
}

unsafe extern "C" fn keyboard_tap(
    _proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    if matches!(
        event_type,
        CG_EVENT_TAP_DISABLED_BY_TIMEOUT | CG_EVENT_TAP_DISABLED_BY_USER_INPUT
    ) {
        crate::diagnostics::record(
            "keyboard_macos_event_tap_disabled",
            &[(
                "reason",
                if event_type == CG_EVENT_TAP_DISABLED_BY_TIMEOUT {
                    "timeout"
                } else {
                    "user-input"
                }
                .to_string(),
            )],
        );
        let tap = EVENT_TAP.load(Ordering::Relaxed);
        if !tap.is_null() {
            unsafe { CGEventTapEnable(tap, true) };
            crate::diagnostics::record("keyboard_macos_event_tap_reenabled", &[]);
        }
        return event;
    }

    if event_type == CG_EVENT_KEY_DOWN
        && unsafe { CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_AUTOREPEAT) } == 0
    {
        let keycode =
            unsafe { CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_KEYCODE) } as u32;
        if !is_modifier_keycode(keycode) {
            if !FIRST_EVENT_RECORDED.swap(true, Ordering::Relaxed) {
                crate::diagnostics::record("keyboard_macos_first_event_received", &[]);
            }
            if let Some(sender) = SENDER.get() {
                let _ = sender.send(KeyEvent::Press);
            }
        }
    }
    event
}
