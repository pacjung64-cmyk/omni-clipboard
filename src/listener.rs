use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, SendInput, UnregisterHotKey, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP,
    MOD_ALT, MOD_CONTROL, MOD_SHIFT, VIRTUAL_KEY, VK_CONTROL, VK_SPACE, VK_V,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowTextW, PostQuitMessage, RegisterClassW,
    HWND_MESSAGE, MSG, WM_CLIPBOARDUPDATE, WM_DESTROY, WM_HOTKEY, WNDCLASSW,
};

pub const HOTKEY_HISTORY_ID: i32 = 1;
pub const HOTKEY_AI_LENS_ID: i32 = 2;

#[derive(Debug, Clone)]
pub enum SystemEvent {
    ClipboardUpdated { text: String, source_app: Option<String> },
    HistoryHotkeyPressed,
    AiLensHotkeyPressed,
}

pub struct Win32Listener {
    running: Arc<AtomicBool>,
}

impl Win32Listener {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start<F>(&self, callback: F) -> Result<(), String>
    where
        F: Fn(SystemEvent) + Send + 'static,
    {
        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);

        std::thread::spawn(move || {
            let class_name: Vec<u16> = "OmniClipListenerClass\0".encode_utf16().collect();

            unsafe {
                let wnd_class = WNDCLASSW {
                    style: 0,
                    lpfnWndProc: Some(wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: 0 as _,
                    hIcon: 0 as _,
                    hCursor: 0 as _,
                    hbrBackground: 0 as _,
                    lpszMenuName: std::ptr::null(),
                    lpszClassName: class_name.as_ptr(),
                };

                RegisterClassW(&wnd_class);

                let hwnd = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    class_name.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    0 as _,
                    0 as _,
                    std::ptr::null(),
                );

                if hwnd.is_null() {
                    return;
                }

                // Register clipboard format listener
                AddClipboardFormatListener(hwnd);

                // Register Hotkeys:
                // 1: Ctrl + Shift + V for Clipboard History
                RegisterHotKey(
                    hwnd,
                    HOTKEY_HISTORY_ID,
                    (MOD_CONTROL | MOD_SHIFT) as u32,
                    VK_V as u32,
                );

                // 2: Alt + Space for Instant AI Lens
                RegisterHotKey(
                    hwnd,
                    HOTKEY_AI_LENS_ID,
                    MOD_ALT as u32,
                    VK_SPACE as u32,
                );

                let mut msg: MSG = std::mem::zeroed();

                while running.load(Ordering::SeqCst) && GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
                    match msg.message {
                        WM_CLIPBOARDUPDATE => {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                if let Ok(text) = clipboard.get_text() {
                                    let app_name = Self::get_foreground_window_title();
                                    callback(SystemEvent::ClipboardUpdated {
                                        text,
                                        source_app: app_name,
                                    });
                                }
                            }
                        }
                        WM_HOTKEY => {
                            let id = msg.wParam as i32;
                            if id == HOTKEY_HISTORY_ID {
                                callback(SystemEvent::HistoryHotkeyPressed);
                            } else if id == HOTKEY_AI_LENS_ID {
                                callback(SystemEvent::AiLensHotkeyPressed);
                            }
                        }
                        _ => {
                            DispatchMessageW(&msg);
                        }
                    }
                }

                RemoveClipboardFormatListener(hwnd);
                UnregisterHotKey(hwnd, HOTKEY_HISTORY_ID);
                UnregisterHotKey(hwnd, HOTKEY_AI_LENS_ID);
                DestroyWindow(hwnd);
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn simulate_paste() {
        unsafe {
            let mut inputs: [INPUT; 4] = std::mem::zeroed();

            // 1. Press CTRL
            inputs[0].r#type = INPUT_KEYBOARD;
            inputs[0].Anonymous.ki.wVk = VK_CONTROL as VIRTUAL_KEY;

            // 2. Press V
            inputs[1].r#type = INPUT_KEYBOARD;
            inputs[1].Anonymous.ki.wVk = VK_V as VIRTUAL_KEY;

            // 3. Release V
            inputs[2].r#type = INPUT_KEYBOARD;
            inputs[2].Anonymous.ki.wVk = VK_V as VIRTUAL_KEY;
            inputs[2].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

            // 4. Release CTRL
            inputs[3].r#type = INPUT_KEYBOARD;
            inputs[3].Anonymous.ki.wVk = VK_CONTROL as VIRTUAL_KEY;
            inputs[3].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;

            SendInput(4, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }

    fn get_foreground_window_title() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }

            let mut buffer = [0u16; 256];
            let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            if len > 0 {
                let os_str = OsString::from_wide(&buffer[..len as usize]);
                return os_str.into_string().ok();
            }
            None
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
