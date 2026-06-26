use std::sync::Mutex;
use std::time::Duration;
use eldenring::util::input::is_key_pressed;
use windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD_DPAD_UP;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB;
use lazy_static::lazy_static;
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use crate::debugging::{add_to_screen_debug, is_debugging, run_every};
use crate::{in_menus, keyboard, PROGRAM_START};
use crate::gamepad::{gamepad_state, BUTTONS};
use crate::keyboard::KEYS;
use crate::settings::Settings;

lazy_static!(
	static ref PREV_BUTTON: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY: Mutex<Option<String>> = Mutex::new(None);
);

pub fn is_player_selecting_spell() -> bool {
	if in_menus() {
		if is_debugging() {
			add_to_screen_debug("Player in menus".to_string());
		}
		return false;
	}
	let settings = Settings::read_or_default();
	match settings.using_controller {
		true => {
			let mut prev_button_mutex = PREV_BUTTON.lock().unwrap();
			let button = settings.button;
			let button_changed = match prev_button_mutex.as_ref() {
				Some(prev_button) => &button == prev_button,
				None => false
			};
			let button_valid = BUTTONS.contains_key(button.to_uppercase().as_str());
			if button_changed && !button_valid {
				tracing::error!("Invalid button: {button}");
			}

			let button_code = *BUTTONS.get(button.to_uppercase().as_str())
				.unwrap_or(&Ok(XINPUT_GAMEPAD_DPAD_UP));
			let pressed = gamepad_state().pressed_duration(button_code);

			pressed.is_some_and(|duration| duration.as_secs_f32() > settings.controller_wheel_open_delay)
		}
		false => {
			let mut prev_key_mutex = PREV_KEY.lock().unwrap();
			let key = settings.key;
			let key_changed = match prev_key_mutex.as_ref() {
				Some(prev_key) => &key == prev_key,
				None => false
			};
			let key_valid = KEYS.contains_key(key.to_uppercase().as_str());
			if key_changed && !key_valid {
				tracing::error!("Invalid key: {key}");
			}

			let key_code = *KEYS.get(key.to_uppercase().as_str())
				.unwrap_or(&VK_TAB.0) as i32;
			let out = keyboard::is_pressed(key_code);
			if is_debugging() {
				run_every!("key valid / pressed" every Duration::from_secs(1) => {
					tracing::info!("{key}: code = {key_code}, valid? = {key_valid}, pressed? = {}, focused? = {}", is_key_pressed(key_code), is_game_focused())
				});
			}
			*prev_key_mutex = Some(key);
			out
		}
	}
}

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}