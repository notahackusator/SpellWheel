use std::sync::Mutex;
use std::time::Duration;
use eldenring::util::input::is_key_pressed;
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_UP};
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VK_CAPITAL, VK_TAB};
use lazy_static::lazy_static;
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use crate::debugging::{add_to_screen_debug, is_debugging, run_every};
use crate::{in_menus, keyboard};
use crate::gamepad::{gamepad_state, BUTTONS};
use crate::keyboard::KEYS;
use crate::rendering::WheelType;
use crate::settings::Settings;

lazy_static!(
	static ref PREV_BUTTON_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_BUTTON_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
);

pub fn selected_wheel_type() -> WheelType {
	if in_menus() {
		if is_debugging() {
			add_to_screen_debug("Player in menus".to_string());
		}
		return WheelType::None;
	}
	
	let mut ret = WheelType::None;
	
	let settings = Settings::read_or_default();
	for (wheel_type, button_mutex, settings_button_default, raw_button_default, key_mutex, settings_key_default, raw_key_default) in [
		(WheelType::Spells, PREV_BUTTON_SPELLS.lock(), settings.spells_button, XINPUT_GAMEPAD_DPAD_UP, PREV_KEY_SPELLS.lock(), settings.spells_key, VK_TAB),
		(WheelType::QuickItems, PREV_BUTTON_QUICK_ITEMS.lock(), settings.quick_items_button, XINPUT_GAMEPAD_DPAD_DOWN, PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key, VK_CAPITAL)
	] {
		match settings.using_controller {
			true => {
				let mut prev_button_mutex = button_mutex.unwrap();
				let button = settings_button_default;
				let button_changed = match prev_button_mutex.as_ref() {
					Some(prev_button) => &button == prev_button,
					None => false
				};
				let button_valid = BUTTONS.contains_key(button.to_uppercase().as_str());
				if button_changed && !button_valid {
					tracing::error!("Invalid button: {button}");
				}

				let button_code = *BUTTONS.get(button.to_uppercase().as_str())
					.unwrap_or(&Ok(raw_button_default));
				let pressed = gamepad_state().pressed_duration(button_code);
				let out = pressed.is_some_and(|duration| duration.as_secs_f32() > settings.controller_wheel_open_delay);
				if is_debugging() {
					run_every!("button valid / pressed" every Duration::from_secs(1) => {
					let button_code = match button_code {
						Ok(code) => code.0.to_string(),
						Err(is_r2) if is_r2 => "R2".to_string(),
						Err(_) => "L2".to_string(),
					};
					tracing::info!("{button}: code = {button_code}, valid? = {button_valid}, pressed? = {}, focused? = {}", out, is_game_focused())
				});
				}
				*prev_button_mutex = Some(button);
				if out {
					ret = wheel_type;
					break;
				}
			}
			false => {
				let mut prev_key_mutex = key_mutex.unwrap();
				let key = settings_key_default;
				let key_changed = match prev_key_mutex.as_ref() {
					Some(prev_key) => &key == prev_key,
					None => false
				};
				let key_valid = KEYS.contains_key(key.to_uppercase().as_str());
				if key_changed && !key_valid {
					tracing::error!("Invalid key: {key}");
				}

				let key_code = *KEYS.get(key.to_uppercase().as_str())
					.unwrap_or(&raw_key_default.0) as i32;
				let out = keyboard::is_pressed(key_code);
				if is_debugging() {
					run_every!("key valid / pressed" every Duration::from_secs(1) => {
					tracing::info!("{key}: code = {key_code}, valid? = {key_valid}, pressed? = {}, focused? = {}", is_key_pressed(key_code), is_game_focused())
				});
				}
				*prev_key_mutex = Some(key);
				if out {
					ret = wheel_type;
					break;
				}
			}
		}
	}
	
	ret
}

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}