use super::{set_status, KeyEvent, STATUS_ACTIVE, STATUS_ERROR};
use std::sync::OnceLock;
use tokio::sync::mpsc::UnboundedSender;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

static SENDER: OnceLock<UnboundedSender<KeyEvent>> = OnceLock::new();

pub(super) fn start(sender: UnboundedSender<KeyEvent>) {
    if SENDER.set(sender).is_err() {
        set_status(STATUS_ERROR);
        return;
    }

    if let Err(error) = std::thread::Builder::new()
        .name("rundev-keyboard-hook".to_string())
        .spawn(move || unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    tracing::error!(%error, "Failed to install keyboard hook");
                    set_status(STATUS_ERROR);
                    return;
                }
            };
            set_status(STATUS_ACTIVE);
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).as_bool() {}
            let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
        })
    {
        tracing::error!(%error, "Failed to start keyboard hook thread");
        set_status(STATUS_ERROR);
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let data = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if data.flags.0 & LLKHF_INJECTED.0 == 0 {
            let event = match wparam.0 as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyEvent::Down(data.vkCode)),
                WM_KEYUP | WM_SYSKEYUP => Some(KeyEvent::Up(data.vkCode)),
                _ => None,
            };
            if let (Some(sender), Some(event)) = (SENDER.get(), event) {
                let _ = sender.send(event);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
