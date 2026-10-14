use crate::debugging::{add_to_screen_debug, is_debugging, run_every};
use crate::gamepad::{BUTTONS, gamepad_state};
use crate::keyboard::KEYS;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::Settings;
use crate::{in_menus, keyboard};
use eldenring::util::input::is_key_pressed;
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CAPITAL, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use std::sync::{LockResult, Mutex, MutexGuard};
use std::time::Duration;
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_BUTTON_FLAGS, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_UP};

lazy_static!(
	static ref PREV_BUTTON_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_BUTTON_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
);

pub fn selected_wheel_type() -> WheelType {
	let pad = unsafe { FD4PadManager::instance().ok()
		.and_then(FD4PadManager::get_in_game_pad) };
	io_selected_wheel_type(pad)
}

pub fn io_selected_wheel_type(pad: Option<&CSInGamePad>) -> WheelType {
	if in_menus() {
		if is_debugging() {
			add_to_screen_debug("Player in menus".to_string());
		}
		return WheelType::None;
	}

	let settings = Settings::read_or_default();

	if (settings.spells_key.to_lowercase() == "ingame" ||
		settings.spells_button.to_lowercase() == "ingame") &&
		pad.is_some_and(|pad| pad.poll_digital_input(UserInputKey::SwitchSpell)) {

		return WheelType::Spells;
	}
	if (settings.quick_items_key.to_lowercase() == "ingame" ||
		settings.quick_items_button.to_lowercase() == "ingame") &&
		pad.is_some_and(|pad| pad.poll_digital_input(UserInputKey::SwitchItem)) {

		return WheelType::QuickItems;
	}

	let spells = (
		WheelType::Spells,
		PREV_BUTTON_SPELLS.lock(), settings.spells_button.clone(), XINPUT_GAMEPAD_DPAD_UP,
		PREV_KEY_SPELLS.lock(), settings.spells_key.clone(), VK_TAB,
	);
	let quick_items = (
		WheelType::QuickItems,
		PREV_BUTTON_QUICK_ITEMS.lock(), settings.quick_items_button.clone(), XINPUT_GAMEPAD_DPAD_DOWN,
		PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key.clone(), VK_CAPITAL,
	);
	for (
		wheel_type,
		prev_button, button, raw_button_default,
		prev_key, key, raw_key_default,
	) in [spells, quick_items] {
		if let Some(value) = is_wheel_open_button_pressed(&settings, wheel_type, prev_button, button, raw_button_default) {
			return value;
		}

		if let Some(value) = is_wheel_open_key_pressed(wheel_type, prev_key, key, &raw_key_default) {
			return value;
		}
	}

	WheelType::None
}

fn is_wheel_open_key_pressed(wheel_type: WheelType, prev_key: LockResult<MutexGuard<Option<String>>>, key: String, raw_key_default: &VIRTUAL_KEY) -> Option<WheelType> {
	let mut prev_key = prev_key.expect("Couldn't acquire prev_key");
	let key_changed = matches!(&*prev_key, Some(prev_key) if prev_key != &key);
	let key_code = *KEYS.get(key.to_uppercase().as_str())
		.unwrap_or_else(|| {
			if key_changed {
				tracing::error!("Invalid key: {key}");
			}
			&raw_key_default.0
		}) as i32;
	*prev_key = Some(key);
	let out = keyboard::is_pressed(key_code);
	if out {
		return Some(wheel_type);
	}
	None
}

fn is_wheel_open_button_pressed(settings: &Settings, wheel_type: WheelType, prev_button: LockResult<MutexGuard<Option<String>>>, button: String, raw_button_default: XINPUT_GAMEPAD_BUTTON_FLAGS) -> Option<WheelType> {
	let mut prev_button = prev_button.expect("Couldn't acquire prev_button");
	let button_changed = matches!(&*prev_button, Some(prev_button) if prev_button != &button);
	let button_code = BUTTONS.get(button.to_uppercase().as_str())
		.cloned()
		.unwrap_or_else(|| {
			if button_changed {
				tracing::error!("Invalid button: {button}");
			}
			Ok(raw_button_default)
		});
	*prev_button = Some(button);
	let pressed = gamepad_state().pressed_duration(button_code);
	let out = pressed.is_some_and(|duration| duration.as_secs_f32() > settings.controller_wheel_open_delay);
	if out {
		return Some(wheel_type);
	}
	None
}

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}