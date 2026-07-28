use super::{set_status, KeyEvent, STATUS_ACTIVE, STATUS_ERROR, STATUS_PERMISSION_REQUIRED};
use sqlx::SqlitePool;
use std::{ffi::c_void, process::Command, sync::OnceLock};
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
const CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
const CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

static SENDER: OnceLock<UnboundedSender<KeyEvent>> = OnceLock::new();

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
    if !has_permission() {
        set_status(STATUS_PERMISSION_REQUIRED);
        let prompted: Option<String> = sqlx::query_scalar(
            "SELECT value FROM app_settings WHERE key = 'keyboard.macos.permission_prompted'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        let granted = unsafe { CGRequestListenEventAccess() };
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
            while !has_permission() {
                set_status(STATUS_PERMISSION_REQUIRED);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            unsafe {
                if SENDER.set(sender).is_err() {
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
                    set_status(STATUS_ERROR);
                    return;
                }
                let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                if source.is_null() {
                    set_status(STATUS_ERROR);
                    return;
                }
                CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);
                set_status(STATUS_ACTIVE);
                CFRunLoopRun();
            }
        })
    {
        tracing::error!(%error, "Failed to start macOS keyboard event tap");
        set_status(STATUS_ERROR);
    }
}

pub(super) fn open_permission_settings() -> Result<(), String> {
    Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
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
    if event_type == CG_EVENT_KEY_DOWN
        && unsafe { CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_AUTOREPEAT) } == 0
    {
        let keycode =
            unsafe { CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_KEYCODE) } as u32;
        if !is_modifier_keycode(keycode) {
            if let Some(sender) = SENDER.get() {
                let _ = sender.send(KeyEvent::Press);
            }
        }
    }
    event
}
