use crate::hwnd;
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow}};
use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
use crate::hwindow::get_window_size;

#[derive(Clone, Copy, Debug)]
pub struct MouseState {
    pub screen: POINT,
    pub client: POINT,
    pub window_focused: bool,
}

impl MouseState {
    pub fn mouse_pos(&self) -> [f32; 2] {
        [self.client.x as f32, self.client.y as f32]
    }
}

pub fn get_mouse_state() -> MouseState {
    let mut screen = POINT::default();
    let mut client;

    unsafe {
        GetCursorPos(&mut screen).ok();
    }

    client = screen;
    unsafe {
        let _ = ScreenToClient(hwnd(), &mut client).ok();
    }

    let focused = unsafe { GetForegroundWindow() == hwnd() };

    MouseState { screen, client, window_focused: focused }
}

pub fn reset_cursor_pos() {
    unsafe {
        let [ww, wh] = get_window_size();
        let mut center = POINT { x: (ww / 2.0) as i32, y: (wh / 2.0) as i32 };
        let _ = ClientToScreen(hwnd(), &mut center);
        let _ = SetCursorPos(center.x, center.y);
    }
}