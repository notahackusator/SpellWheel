use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClientRect, GetWindowThreadProcessId, IsWindowVisible};
use crate::hwnd;

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let target = &mut *(lparam.0 as *mut Option<HWND>);

    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == GetCurrentProcessId() && IsWindowVisible(hwnd).as_bool() {
        *target = Some(hwnd);
        return BOOL(0); // stop enumeration
    }

    BOOL(1) // continue
}

pub fn get_process_window() -> Option<HWND> {
    let mut hwnd: Option<HWND> = None;
    unsafe {
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut hwnd as *mut _ as isize),
        ).ok();
    }
    hwnd
}

pub fn get_window_size() -> [f32; 2] {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd(), &mut rect).ok() };
    // rect.left and rect.top are always 0 for client rects
    [rect.right as f32, rect.bottom as f32]
}