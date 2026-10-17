use crate::debugging::{add_to_screen_debug, is_debugging, run_every, run_once};
use crate::gamepad::{BUTTONS, gamepad_state};
use crate::keyboard::KEYS;
use crate::keyboard::listener::{Listener, static_listener};
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::Settings;
use crate::{in_menus, keyboard};
use bindings::BindingEntry;
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CAPITAL, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use retour::GenericDetour;
use std::mem;
use std::sync::{LockResult, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VSC_TO_VK_EX, MapVirtualKeyW, VK_0};
use windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD_BUTTON_FLAGS;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::io::bindings::Bindings;

mod bindings;

lazy_static!(
	// static ref PREV_BUTTON_SPELLS: Mutex<Option<String>> = Mutex::new(None);
	// static ref PREV_BUTTON_QUICK_ITEMS: Mutex<Option<String>> = Mutex::new(None);
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
	if let Some(pad) = pad.as_ref() {
		let bindings = Bindings::from_pad(pad);
		let ss = bindings.get_input_bindings(pad, &UserInputKey::SwitchSpell);
		let si = bindings.get_input_bindings(pad, &UserInputKey::SwitchItem);
		add_to_screen_debug(format!("  ss={ss:?} si={si:?}"));
	}

	let settings = Settings::read_or_default();

	if (settings.spells_key.to_lowercase() == "ingame"/* ||
		settings.spells_button.to_lowercase() == "ingame"*/) &&
		pad.is_some_and(|pad| pad.poll_digital_input(UserInputKey::SwitchSpell)) {

		return WheelType::Spells;
	}
	if (settings.quick_items_key.to_lowercase() == "ingame"/* ||
		settings.quick_items_button.to_lowercase() == "ingame"*/) &&
		pad.is_some_and(|pad| pad.poll_digital_input(UserInputKey::SwitchItem)) {

		return WheelType::QuickItems;
	}

	let spells = (
		WheelType::Spells,
		PREV_KEY_SPELLS.lock(), settings.spells_key.clone(), VK_TAB,
	);
	let quick_items = (
		WheelType::QuickItems,
		PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key.clone(), VK_CAPITAL,
	);
	for (
		wheel_type,
		prev_key, key, raw_key_default,
	) in [spells, quick_items] {
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

pub fn is_game_focused() -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(foreground, Some(&mut pid));
        pid == GetCurrentProcessId()
    }
}

type GenericHookFn = unsafe extern "C" fn(usize, usize, usize, usize) -> usize;

const INPUT_CHECK_PREFIX: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x83, 0xEC, 0x78, 0x49, 0xC7, 0x43, 0xA8];
const SWITCH_HOLD_PREFIX: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x81, 0xEC, 0x88, 0x00, 0x00, 0x00, 0x49, 0xC7, 0x43, 0x98];

struct Target {
	name: &'static str,
	rva: usize,
	prefix: &'static [u8],
	/// If set, only suppress when the 2nd argument (rdx) is one of these ids.
	/// The hold check is one function shared by several actions, with the
	/// action id as its second argument (13 = spell, 14 = item).
	ids: Option<&'static [u32]>,
	suppression_type: SuppressionType,
}

enum SuppressionType {
	Spell,
	Item,
	Both,
}

const NUM_HOOKS: usize = 5;
const TARGETS: [Target; NUM_HOOKS] = [
	// tap -> next slot
	Target {
		name: "Switch Spell request",
		rva: 0x759120,
		prefix: INPUT_CHECK_PREFIX,
		ids: None,
		suppression_type: SuppressionType::Spell,
	},
	Target {
		name: "Switch Item request",
		rva: 0x7590B0,
		prefix: INPUT_CHECK_PREFIX,
		ids: None,
		suppression_type: SuppressionType::Spell,
	},
	// hold -> return to first slot
	Target {
		name: "Switch Spell hold",
		rva: 0x759680,
		prefix: INPUT_CHECK_PREFIX,
		ids: None,
		suppression_type: SuppressionType::Item,
	},
	Target {
		name: "Switch Item hold",
		rva: 0x7593D0,
		prefix: INPUT_CHECK_PREFIX,
		ids: None,
		suppression_type: SuppressionType::Item,
	},
	// hold -> previous slot (shared function, filtered by action id)
	Target {
		name: "Switch Spell / Item hold",
		rva: 0x759270,
		prefix: SWITCH_HOLD_PREFIX,
		ids: Some(&[13, 14]),
		suppression_type: SuppressionType::Both,
	},
];

static HOOKS: [OnceLock<GenericDetour<GenericHookFn>>; NUM_HOOKS] = [const { OnceLock::new() }; NUM_HOOKS];

unsafe extern "C" fn generic_hook<const I: usize>(a: usize, b: usize, c: usize, d: usize) -> usize {
	let hit = match TARGETS.get(I).and_then(|t| t.ids) {
		None => true,
		Some(ids) => ids.contains(&(b as u32)),
	};
	if hit {
		return 0;
	}

	let res = HOOKS[I].get().unwrap().call(a, b, c, d);

	res
}

const HOOK_FNS: [GenericHookFn; NUM_HOOKS] = [
	generic_hook::<0>, generic_hook::<1>, generic_hook::<2>, generic_hook::<3>, generic_hook::<4>
];

fn module_base() -> usize {
	unsafe { GetModuleHandleW(std::ptr::null()) as usize }
}

pub fn install_io_hooks() {
	unsafe {
		let base = module_base();

		for (i, target) in TARGETS.iter().enumerate() {
			if HOOKS[i].get().is_some() {
				continue;
			}

			let addr = base + target.rva;

			let bytes = std::slice::from_raw_parts(addr as *const u8, target.prefix.len());
			if bytes != target.prefix {
				let expected: String = target.prefix.iter().map(|b| format!("{b:#02x}")).collect();
				let found: String = bytes.iter().map(|b| format!("{b:#02x}")).collect();
				tracing::warn!(
					"Couldn't hook {}: prologue mismatch at rva {:#x}: Expected 0x{expected}, found 0x{found}",
					target.name, target.rva,
				);
				continue;
			}

			let target_fn: GenericHookFn = mem::transmute::<usize, GenericHookFn>(addr);
			let detour = match GenericDetour::<GenericHookFn>::new(target_fn, HOOK_FNS[i]) {
				Ok(detour) => detour,
				Err(err) => {
					tracing::warn!("Couldn't hook {}: create failed: {err}", target.name);
					continue;
				}
			};

			let _ = HOOKS[i].set(detour);

			match HOOKS[i].get().unwrap().enable() {
				Ok(()) => tracing::info!("{} hooked", target.name),
				Err(e) => tracing::warn!("{}: enable failed: {e}", target.name),
			}
		}
	}
}

pub fn uninstall_io_hooks() {
	unsafe {
		for h in HOOKS.iter().filter_map(|h| h.get()) {
			let _ = h.disable();
		}
	}
}