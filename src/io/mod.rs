use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::gamepad::set_last_open_is_controller;
use crate::io::bindings::{set_spell_bindings, Bindings, set_item_bindings, PhysicalKeys, PressedStatus};
use crate::keyboard::KEYS;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::{InputType, Settings};
use crate::{in_menus, keyboard};
use bindings::BindingEntry;
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CAPITAL, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_0;
use crate::keyboard::listener::{static_listener, Listener};

pub mod bindings;
pub mod hook;
pub mod classes;

lazy_static!(
	static ref PREV_KEY_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_KEY_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
	static ref PREV_SPELL_STATUS: Arc<Mutex<PressedStatus>> = Arc::new(Mutex::new(PressedStatus::none()));
	static ref PREV_ITEM_STATUS: Arc<Mutex<PressedStatus>> = Arc::new(Mutex::new(PressedStatus::none()));
	static ref GLOBAL_SPELL_STATUS: Arc<Mutex<PressedStatus>> = Arc::new(Mutex::new(PressedStatus::none()));
	static ref GLOBAL_ITEM_STATUS: Arc<Mutex<PressedStatus>> = Arc::new(Mutex::new(PressedStatus::none()));
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

	if let Some(pad) = pad.as_ref() {
		let bindings = Bindings::from_pad(pad);

		let switch_spell = bindings.get_input_bindings(pad, &UserInputKey::SwitchSpell);
		let switch_item = bindings.get_input_bindings(pad, &UserInputKey::SwitchItem);
		let current_spell_status = switch_spell.pressed_status(None);
		let current_item_status = switch_item.pressed_status(None);
		if is_debugging() {
			add_to_screen_debug(format!("Switch spell inputs: {switch_spell:?}"));
			add_to_screen_debug(format!("  Pressed status: {current_spell_status:?}"));
			add_to_screen_debug(format!("Switch item inputs: {switch_item:?}"));
			add_to_screen_debug(format!("  Pressed status: {current_item_status:?}"));
		}
		set_spell_bindings(switch_spell);
		set_item_bindings(switch_item);
		let mut prev_spell_status = PREV_SPELL_STATUS.lock()
			.expect("Couldn't acquire PREV_SPELL_STATUS");
		let mut prev_item_status = PREV_ITEM_STATUS.lock()
			.expect("Couldn't acquire PREV_ITEM_STATUS");
		let mut global_spell_status = GLOBAL_SPELL_STATUS.lock()
			.expect("Couldn't acquire GLOBAL_SPELL_STATUS");
		let mut global_item_status = GLOBAL_ITEM_STATUS.lock()
			.expect("Couldn't acquire GLOBAL_ITEM_STATUS");
		let spell_status = if let InputType::Hold = settings.spell_wheel_input_type() {
			current_spell_status
		} else {
			global_spell_status.flip_if(
				current_spell_status.mouse() && !prev_spell_status.mouse(),
				current_spell_status.pad() && !prev_spell_status.pad(),
				current_spell_status.key() && !prev_spell_status.key(),
				current_spell_status.scroll() && !prev_spell_status.scroll()
			);
			global_spell_status.clone()
		};
		let item_status = if let InputType::Hold = settings.item_wheel_input_type() {
			current_item_status
		} else {
			global_item_status.flip_if(
				current_item_status.mouse() && !prev_item_status.mouse(),
				current_item_status.pad() && !prev_item_status.pad(),
				current_item_status.key() && !prev_item_status.key(),
				current_item_status.scroll() && !prev_item_status.scroll()
			);
			global_item_status.clone()
		};
		*prev_spell_status = current_spell_status;
		*prev_item_status = current_item_status;

		if settings.spell_wheel_enabled {
			if (spell_status.key() || spell_status.mouse()) && settings.spells_key.to_lowercase() == "ingame" {
				set_last_open_is_controller(false);
				return WheelType::Spells;
			}
			if spell_status.pad() {
				set_last_open_is_controller(true);
				return WheelType::Spells;
			}
		}

		if settings.item_wheel_enabled {
			if (item_status.key() || item_status.mouse()) && settings.quick_items_key.to_lowercase() == "ingame" {
				set_last_open_is_controller(false);
				return WheelType::QuickItems;
			}
			if item_status.pad() {
				set_last_open_is_controller(true);
				return WheelType::QuickItems;
			}
		}
	}

	let spells = (
		WheelType::Spells, settings.spell_wheel_enabled,
		PREV_KEY_SPELLS.lock(), settings.spells_key.clone(), VK_TAB,
	);
	let quick_items = (
		WheelType::QuickItems, settings.item_wheel_enabled,
		PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key.clone(), VK_CAPITAL,
	);
	for (
		wheel_type, enabled,
		prev_key, key, raw_key_default,
	) in [spells, quick_items] {
		if !enabled {
			continue;
		}
		if let Some(value) = is_wheel_open_key_pressed(wheel_type, prev_key, key, &raw_key_default) {
			set_last_open_is_controller(false);
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

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

