use objc2_app_kit::NSWorkspace;
use std::{ffi::c_void, ptr, time::Duration};

use super::PlatformSnapshot;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    fn CGSessionCopyCurrentDictionary() -> *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        allocator: *const c_void,
        string: *const i8,
        encoding: u32,
    ) -> *const c_void;
    fn CFDictionaryGetValue(dictionary: *const c_void, key: *const c_void) -> *const c_void;
    fn CFBooleanGetValue(boolean: *const c_void) -> u8;
    fn CFRelease(value: *const c_void);
}

pub(super) fn snapshot() -> PlatformSnapshot {
    let app_identifier = foreground_bundle_identifier();
    let locked = screen_is_locked()
        || app_identifier
            .as_deref()
            .is_some_and(|identifier| identifier == "com.apple.loginwindow");

    PlatformSnapshot {
        app_identifier,
        idle_for: input_idle_time(),
        locked,
    }
}

fn screen_is_locked() -> bool {
    const UTF8_ENCODING: u32 = 0x0800_0100;
    const SCREEN_LOCKED_KEY: &[u8] = b"CGSSessionScreenIsLocked\0";

    unsafe {
        let dictionary = CGSessionCopyCurrentDictionary();
        if dictionary.is_null() {
            return false;
        }
        let key = CFStringCreateWithCString(
            ptr::null(),
            SCREEN_LOCKED_KEY.as_ptr().cast(),
            UTF8_ENCODING,
        );
        if key.is_null() {
            CFRelease(dictionary);
            return false;
        }
        let value = CFDictionaryGetValue(dictionary, key);
        let locked = !value.is_null() && CFBooleanGetValue(value) != 0;
        CFRelease(key);
        CFRelease(dictionary);
        locked
    }
}

fn foreground_bundle_identifier() -> Option<String> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace
        .frontmostApplication()?
        .bundleIdentifier()
        .map(|identifier| identifier.to_string().to_ascii_lowercase())
}

fn input_idle_time() -> Duration {
    // kCGEventSourceStateCombinedSessionState = 0, kCGAnyInputEventType = ~0.
    let seconds = unsafe { CGEventSourceSecondsSinceLastEventType(0, u32::MAX) };
    if seconds.is_finite() && seconds >= 0.0 {
        Duration::from_secs_f64(seconds)
    } else {
        Duration::MAX
    }
}
