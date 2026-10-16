use std::collections::HashMap;
use std::mem;
use crate::debugging::{add_to_screen_debug, is_debugging, run_every, run_once};
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
use std::sync::{Arc, LockResult, Mutex, MutexGuard, OnceLock, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::{FD4PadManager, InputTypeGroup};
use fromsoftware_shared::FromStatic;
use retour::GenericDetour;
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_BUTTON_FLAGS, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_UP};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

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

type GenFn = unsafe extern "C" fn(usize, usize, usize, usize) -> usize;

lazy_static!(
	static ref DISABLE_VANILLA_SWITCH: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
	static ref DEBUG_LOG: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
);

pub fn set_disable_vanilla_switch(on: bool) {
	DISABLE_VANILLA_SWITCH.store(on, Ordering::Relaxed);
}

pub fn set_debug_log(on: bool) {
	DEBUG_LOG.store(on, Ordering::Relaxed);
}


const INPUT_CHECK_78: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x83, 0xEC, 0x78, 0x49, 0xC7, 0x43, 0xA8];
const SWITCH_HOLD: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x81, 0xEC, 0x88, 0x00, 0x00, 0x00, 0x49, 0xC7, 0x43, 0x98];

struct Target {
	name: &'static str,
	rva: usize,
	prefix: &'static [u8],
	/// If set, only suppress when the 2nd argument (rdx) is one of these ids.
	/// The hold check is one function shared by several actions, with the
	/// action id as its second argument (13 = spell, 14 = item).
	ids: Option<&'static [u32]>,
}

const TARGETS: &[Target] = &[
	// tap -> next slot
	Target { name: "SwitchSpell request", rva: 0x759120, prefix: INPUT_CHECK_78, ids: None },
	Target { name: "SwitchItem request",  rva: 0x7590B0, prefix: INPUT_CHECK_78, ids: None },
	// held repeat
	Target { name: "SwitchSpell repeat",  rva: 0x759680, prefix: INPUT_CHECK_78, ids: None },
	Target { name: "SwitchItem repeat",   rva: 0x7593D0, prefix: INPUT_CHECK_78, ids: None },
	// hold -> previous slot (shared function, filtered by action id)
	Target { name: "Switch hold",         rva: 0x759270, prefix: SWITCH_HOLD,    ids: Some(&[13, 14]) },
];

const NUM_HOOKS: usize = 8;
static HOOKS: [OnceLock<GenericDetour<GenFn>>; NUM_HOOKS] = [const { OnceLock::new() }; NUM_HOOKS];
static NAMES: [OnceLock<&'static str>; NUM_HOOKS] = [const { OnceLock::new() }; NUM_HOOKS];

unsafe extern "C" fn detour<const I: usize>(a: usize, b: usize, c: usize, d: usize) -> usize {
	if DISABLE_VANILLA_SWITCH.load(Ordering::Relaxed) {
		let hit = match TARGETS.get(I).and_then(|t| t.ids) {
			None => true,
			Some(ids) => ids.contains(&(b as u32)),
		};
		if hit {
			return 0;
		}
	}

	let res = HOOKS[I].get().unwrap().call(a, b, c, d);

	if DEBUG_LOG.load(Ordering::Relaxed) && (res & 0xFF) != 0 {
		tracing::info!(
            "{} (arg2={b}) returned {res:#x}",
            NAMES[I].get().copied().unwrap_or("?")
        );
	}
	res
}

const DETOURS: [GenFn; NUM_HOOKS] = [
	detour::<0>, detour::<1>, detour::<2>, detour::<3>, detour::<4>, detour::<5>, detour::<6>, detour::<7>
];

fn module_base() -> usize {
	unsafe { GetModuleHandleW(std::ptr::null()) as usize }
}

pub unsafe fn install_all() -> usize {
	let base = module_base();
	let mut ok = 0;

	for (i, target) in TARGETS.iter().enumerate().take(NUM_HOOKS) {
		if HOOKS[i].get().is_some() {
			continue;
		}

		let addr = base + target.rva;

		let bytes = std::slice::from_raw_parts(addr as *const u8, target.prefix.len());
		if bytes != target.prefix {
			tracing::warn!(
				"Couldn't hook {}: prologue mismatch at rva {:#x}: Expected 0x{}, found 0x{}",
				target.name, target.rva,
				target.prefix.iter().map(|b| format!("{b:#02x}")).collect::<String>(),
				bytes.iter().map(|b| format!("{b:#02x}")).collect::<String>()
			);
			continue;
		}

		let target_fn: GenFn = mem::transmute::<usize, GenFn>(addr);
		let detour = match GenericDetour::<GenFn>::new(target_fn, DETOURS[i]) {
			Ok(detour) => detour,
			Err(err) => {
				tracing::warn!("Couldn't hook {}: create failed: {err}", target.name);
				continue;
			}
		};

		let _ = NAMES[i].set(target.name);
		let _ = HOOKS[i].set(detour);

		match HOOKS[i].get().unwrap().enable() {
			Ok(()) => {
				ok += 1;
				tracing::info!("{} hooked", target.name);
			}
			Err(e) => tracing::warn!("{}: enable failed: {e}", target.name),
		}
	}
	ok
}

/// Disable all hooks. Call before the DLL unloads.
pub unsafe fn uninstall_all() {
	for h in HOOKS.iter().filter_map(|h| h.get()) {
		let _ = unsafe { h.disable() };
	}
}

// ---- usage -----------------------------------------------------------------
//
//   unsafe { switch_hooks::install_all() };
//
//   // first run: see which checks fire for which button
//   switch_hooks::set_debug_log(true);
//
//   // when the "use in-game key bindings" option is on:
//   switch_hooks::set_disable_vanilla_switch(true);

/*
// lazy_static!(
// 	// static ref PREV_CONTROLS: Arc<RwLock<[Option<InputTypeGroup>; 4]>> = Arc::new(RwLock::new([None; 4]));
// 	static ref BYTE_SNAPSHOTS: Arc<RwLock<HashMap<String, Vec<u8>>>> = Arc::new(RwLock::new(HashMap::new()));
// );
//
// unsafe fn snapshot<T>(location: &T) -> Vec<u8> {
// 	std::slice::from_raw_parts(location as *const T as *const u8, 200).to_vec()
// }
//
// unsafe fn store_snapshot<T>(name: &str, location: &T) {
// 	let snapshot = snapshot(location);
// 	let mut map = BYTE_SNAPSHOTS.write().expect("Couldn't acquire BYTE_SNAPSHOTS");
// 	let prev = map.get(&format!("{name} {location:p}"));
// 	if let Some(prev) = prev {
// 		let mut diffs = String::new();
// 		for (i, (a, b)) in prev.iter().zip(snapshot.iter()).enumerate() {
// 			if a != b {
// 				diffs.push_str(&format!("{i}: {a} -> {b}  "));
// 			}
// 		}
// 		if !diffs.is_empty() {
// 			tracing::info!("Diff {name} {location:p}:");
// 			tracing::info!("  {diffs}");
// 		}
// 	}
// 	map.insert(format!("{name} {location:p}"), snapshot);
// }

pub fn dbg_controls() {
	unsafe {
		let Some(pad_man) = FD4PadManager::instance().ok() else {
			return;
		};
		for (i, device) in pad_man.pad_device_list.iter().enumerate() {
			let device = device.as_ref();
			let multi = device.virtual_multi_device.as_ref();
			let vid = &multi.virtual_input_data;
			let bitset = std::slice::from_raw_parts(vid.dynamic_bitset.bitset.as_ptr(), vid.dynamic_bitset.integer_count);
			let mut str = format!("  Device {i} Bitset: ");
			for uint in bitset {
				str.push_str(&format!("{uint:b}"));
			}
			add_to_screen_debug(str);
		}
	}
}

fn dbg_controls_old() {
	unsafe {
		if let Some(pad_man) = FD4PadManager::instance().ok() {
			let Some(pad) = pad_man.get_in_game_pad() else {
				return;
			};
			// run_every!("switch ddddddd" every Duration::from_secs(3) => {
			// 	pad.allow_polling = !pad.allow_polling;
			// });
			// pad.input_code_check
			// add_to_screen_debug(format!("Polling: {}", pad.allow_polling));
			let vid = &pad.pad_device.as_ref().virtual_multi_device.as_ref().virtual_input_data;
			for (i, key_assign) in pad_man.key_assign_map_list.iter().enumerate() {
				let key_assign = key_assign.as_ref();
				add_to_screen_debug(format!("  {i}:"));
				for (j, entry) in key_assign.iter().enumerate() {
					add_to_screen_debug(format!("    {j}: {}={:p}", entry.first, entry.second));
					if let Some(itg) = pad.input_type_group.as_ref().find(&UserInputKey::SwitchSpell) {
						add_to_screen_debug(format!("     itg: {itg:?}"));
						if let Some(idx) = entry
							.second
							.as_ref()
							.virtual_input_data_index_map
							.as_ref()
							.find(&itg.mapped_input_list[0]) {
							add_to_screen_debug(format!("     idk: {}", vid.get_digital(*idx as usize)));
						}
					}
					for (l, entry) in entry.second.as_ref().virtual_input_data_index_map.as_ref().iter().enumerate() {
						add_to_screen_debug(format!("      {l}: {}={}={}", entry.first, entry.second, vid.get_digital(entry.second as usize)));
					}
				}
			}
			// let key_assign = pad.key_assign.as_ref();
			// if (key_assign as *const _) as usize == 0 || key_assign as *const _ as usize % 8 != 0 { tracing::warn!("key_assign bad: {key_assign:p}"); return; }
			// let input_mapper_ptr_ptr = pad.key_assign.add(size_of::<usize>() * 2).as_ptr() as *const usize;
			// if (input_mapper_ptr_ptr as *const _) as usize == 0 || input_mapper_ptr_ptr as *const _ as usize % 8 != 0 { tracing::warn!("input_mapper_ptr_ptr bad: {input_mapper_ptr_ptr:p}"); return; }
			// let input_mapper_ptr = *input_mapper_ptr_ptr as *const ();
			// if (input_mapper_ptr as *const _) as usize == 0 || input_mapper_ptr as *const _ as usize % 8 != 0 { tracing::warn!("input_mapper_ptr bad: {input_mapper_ptr:p}"); return; }
			// let input_mapper = input_mapper_ptr.as_ref_unchecked();
			// if (input_mapper as *const _) as usize == 0 || input_mapper as *const _ as usize % 8 != 0 { tracing::warn!("input_mapper bad: {input_mapper:p}"); return; }
			//
			// store_snapshot("key assign", key_assign);
			// store_snapshot("input mapper", input_mapper);
			// for pad_device in pad_man.pad_device_list.iter() {
			// 	let pad_device = pad_device.as_ref();
			// 	if (pad_device as *const _) as usize == 0 || pad_device as *const _ as usize % 8 != 0 { tracing::warn!("pad_device bad: {pad_device:p}"); return; }
			// 	let virtual_multi_device = pad_device.virtual_multi_device.as_ref();
			// 	if (virtual_multi_device as *const _) as usize == 0 || virtual_multi_device as *const _ as usize % 8 != 0 { tracing::warn!("virtual_multi_device bad: {virtual_multi_device:p}"); return; }
			// 	let virtual_input_data = &pad_device.virtual_multi_device.as_ref().virtual_input_data;
			// 	if (virtual_input_data as *const _) as usize == 0 || virtual_input_data as *const _ as usize % 8 != 0 { tracing::warn!("virtual_input_data bad: {virtual_input_data:p}"); return; }
			//
			// 	store_snapshot("pad device", pad_device);
			// 	store_snapshot("virtual multi device", virtual_multi_device);
			// 	store_snapshot("virtual input data", virtual_input_data);
			// }
			// run_every!("Sonion ring" every Duration::from_secs(1) => {
			// 	let key_assign = pad.key_assign.as_ref();
			// 	let x = key_assign.virtual_input_data_index_map.as_ref().find(&811);
			// 	let y = key_assign.virtual_input_data_index_map.as_ref().find(&815);
			// 	tracing::info!("zeppelin ecato mexico mertad: {x:?} {y:?}");
			// });
			// let type_group = unsafe { pad.input_type_group.as_ref() };
			// let mut prev_controls = PREV_CONTROLS.write().expect("Couldn't acquire PREV_CONTROLS");
			// let controls = [
			// 	type_group.find(&UserInputKey::SwitchItem).cloned(),
			// 	type_group.find(&UserInputKey::SwitchItem2).cloned(),
			// 	type_group.find(&UserInputKey::SwitchSpell).cloned(),
			// 	type_group.find(&UserInputKey::SwitchSpell2).cloned(),
			// ];
			// for i in 0..4 {
			// 	if mem::transmute::<Option<InputTypeGroup>, [u128; 2]>(prev_controls[i]) !=
			// 		mem::transmute::<Option<InputTypeGroup>, [u128; 2]>(controls[i]) {
			// 		tracing::info!("Different {i}: {:?}", controls[i]);
			// 	}
			// }
			// *prev_controls = controls;
		}
	}
}*/