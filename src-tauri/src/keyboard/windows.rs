use super::{set_status, KeyEvent, STATUS_ACTIVE, STATUS_ERROR};
use std::{mem::size_of, sync::OnceLock};
use tokio::sync::mpsc::UnboundedSender;
use windows::{
    core::w,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::{
                GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
                RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEKEYBOARD,
            },
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
                RegisterClassW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
                HWND_MESSAGE, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
            },
        },
    },
};

static SENDER: OnceLock<UnboundedSender<KeyEvent>> = OnceLock::new();

pub(super) fn start(sender: UnboundedSender<KeyEvent>) {
    if SENDER.set(sender).is_err() {
        set_status(STATUS_ERROR);
        return;
    }

    if let Err(error) = std::thread::Builder::new()
        .name("rundev-keyboard-raw-input".to_string())
        .spawn(move || unsafe {
            if let Err(error) = run_message_window() {
                tracing::error!(%error, "Failed to start Windows raw keyboard input");
                set_status(STATUS_ERROR);
            }
        })
    {
        tracing::error!(%error, "Failed to start raw keyboard input thread");
        set_status(STATUS_ERROR);
    }
}

unsafe fn run_message_window() -> Result<(), String> {
    let module = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
    let instance = HINSTANCE(module.0);
    let class_name = w!("RunDevKeyboardRawInput");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: class_name,
        ..Default::default()
    };
    if unsafe { RegisterClassW(&window_class) } == 0 {
        return Err(windows::core::Error::from_win32().to_string());
    }

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("RunDev Keyboard Input"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|error| error.to_string())?;

    let keyboard = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x06,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: window,
    };
    unsafe { RegisterRawInputDevices(&[keyboard], size_of::<RAWINPUTDEVICE>() as u32) }
        .map_err(|error| error.to_string())?;
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            Some(instance),
            0,
        )
    }
    .map_err(|error| error.to_string())?;

    set_status(STATUS_ACTIVE);
    let mut message = MSG::default();
    loop {
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 <= 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    let _ = unsafe { UnhookWindowsHookEx(hook) };
    Ok(())
}

unsafe extern "system" fn low_level_keyboard_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let keyboard = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        send_key_event(wparam.0 as u32, keyboard.vkCode);
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_INPUT {
        unsafe { handle_raw_input(lparam) };
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

unsafe fn handle_raw_input(lparam: LPARAM) {
    let mut size = 0_u32;
    let header_size = size_of::<RAWINPUTHEADER>() as u32;
    let input = HRAWINPUT(lparam.0 as *mut core::ffi::c_void);
    if unsafe { GetRawInputData(input, RID_INPUT, None, &mut size, header_size) } == u32::MAX
        || size < size_of::<RAWINPUT>() as u32
    {
        return;
    }

    let mut buffer = vec![0_u8; size as usize];
    if unsafe {
        GetRawInputData(
            input,
            RID_INPUT,
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            header_size,
        )
    } == u32::MAX
    {
        return;
    }

    let raw = unsafe { &*(buffer.as_ptr().cast::<RAWINPUT>()) };
    if raw.header.dwType != RIM_TYPEKEYBOARD.0 {
        return;
    }
    let keyboard = unsafe { raw.data.keyboard };
    send_key_event(keyboard.Message, keyboard.VKey as u32);
}

fn send_key_event(message: u32, key: u32) {
    let event = match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyEvent::Down(key)),
        WM_KEYUP | WM_SYSKEYUP => Some(KeyEvent::Up(key)),
        _ => None,
    };
    if let (Some(sender), Some(event)) = (SENDER.get(), event) {
        let _ = sender.send(event);
    }
}
