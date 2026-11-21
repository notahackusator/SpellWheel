use std::cell::OnceCell;
use std::mem;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use retour::GenericDetour;
use std::sync::OnceLock;
use std::time::Instant;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::debugging::add_to_screen_debug;
use crate::game_interface::poll_use_quick_item;
use crate::io::bindings::{get_item_bindings, get_spell_bindings};
use crate::settings::{InputType, Settings};

type GenericHookFn = unsafe extern "C" fn(usize, usize, usize, usize) -> usize;

const INPUT_CHECK_PREFIX: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x83, 0xEC, 0x78, 0x49, 0xC7, 0x43, 0xA8];
const SWITCH_HOLD_PREFIX: &[u8] = &[0x4C, 0x8B, 0xDC, 0x48, 0x81, 0xEC, 0x88, 0x00, 0x00, 0x00, 0x49, 0xC7, 0x43, 0x98];

struct InputHookTarget {
	name: &'static str,
	rva: usize,
	prefix: &'static [u8],
	/// If set, only suppress when the 2nd argument (rdx) is one of these ids.
	/// The hold check is one function shared by several actions, with the
	/// action id as its second argument (13 = spell, 14 = item).
	ids: Result<SuppressionType, &'static [(u32, SuppressionType)]>,
	controller_suppression_length: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuppressionType {
	Spell,
	Item,
	Both,
}

const NUM_HOOKS: usize = 5;
const INPUT_HOOK_TARGETS: [InputHookTarget; NUM_HOOKS] = [
	InputHookTarget {
		name: "Switch Spell request",
		rva: 0x7595D0,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Spell),
		controller_suppression_length: 0.0,
	},
	InputHookTarget {
		name: "Switch Item request",
		rva: 0x759560,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Item),
		controller_suppression_length: 0.0,
	},
	InputHookTarget {
		name: "Switch Spell repeat",
		rva: 0x759B30,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Spell),
		controller_suppression_length: 0.5,
	},
	InputHookTarget {
		name: "Switch Item repeat",
		rva: 0x759880,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Item),
		controller_suppression_length: 0.5,
	},
	InputHookTarget {
		name: "Switch Spell / Item hold",
		rva: 0x759720,
		prefix: SWITCH_HOLD_PREFIX,
		ids: Err(&[
			(13, SuppressionType::Spell),
			(14, SuppressionType::Item),
		]),
		controller_suppression_length: 0.5,
	},
];
static INPUT_HOOKS: [OnceLock<GenericDetour<GenericHookFn>>; NUM_HOOKS] = [const { OnceLock::new() }; NUM_HOOKS];

unsafe extern "C" fn generic_hook<const I: usize>(a: usize, b: usize, c: usize, d: usize) -> usize {
	let settings = Settings::read_or_default();
	let items_key_ingame = settings.quick_items_key.to_lowercase() == "ingame";
	let spells_key_ingame = settings.spells_key.to_lowercase() == "ingame";

	let target = &INPUT_HOOK_TARGETS[I];

	let items_pressed = get_item_bindings()
		.pressed_status(Some(target.controller_suppression_length));
	let spells_pressed = get_spell_bindings()
		.pressed_status(Some(target.controller_suppression_length));

	let supp_type = match target.ids {
		Ok(supp_type) => supp_type,
		Err(supp_types) => supp_types.iter()
			.find(|(id, _)| *id == b as u32)
			.map(|(_, supp_type)| *supp_type)
			.unwrap_or(SuppressionType::Both),
	};

	let mut ignore = true;
	match supp_type {
		SuppressionType::Item => {
			if !settings.item_wheel_enabled {
				ignore = false;
			}
			// Cycle if mouse button is scroll AND (scrolling doesn't toggle wheel OR input type is not toggle)
			if items_pressed.scroll() && (!settings.scrolling_toggles_wheel || settings.item_wheel_input_type() == InputType::Hold) {
				ignore = false;
			}
			// Cycle if pressed in-game key while control is not in-game
			if (items_pressed.key() || items_pressed.mouse()) && !items_key_ingame {
				ignore = false;
			}
			// Cycle if pressed button for less than opening time
			if items_pressed.pad() {
				ignore = false;
			}
		}
		SuppressionType::Spell => {
			if !settings.spell_wheel_enabled {
				ignore = false;
			}
			// Cycle if mouse button is scroll AND (scrolling doesn't toggle wheel OR input type is not toggle)
			if spells_pressed.scroll() && (!settings.scrolling_toggles_wheel || settings.spell_wheel_input_type() == InputType::Hold) {
				ignore = false;
			}
			// Cycle if pressed in-game key while control is not in-game
			if (spells_pressed.key() || spells_pressed.mouse()) && !spells_key_ingame {
				ignore = false;
			}
			// Cycle if pressed button for less than opening time
			if spells_pressed.pad() {
				ignore = false;
			}
		}
		_ => {}
	}
	if ignore {
		return 0;
	}

	let res = INPUT_HOOKS[I].get().unwrap().call(a, b, c, d);

	res
}

const INPUT_HOOK_FNS: [GenericHookFn; NUM_HOOKS] = [
	generic_hook::<0>, generic_hook::<1>, generic_hook::<2>, generic_hook::<3>, generic_hook::<4>
];

const ITEM_HOOK_PREFIX: &[u8] = &[0x40, 0x53, 0x48, 0x83, 0xec, 0x20, 0x48, 0x8d, 0x81, 0x90, 0x07];

struct ItemHookTarget {
	name: &'static str,
	rva: usize,
	prefix: &'static [u8],
}

const ITEM_HOOK_TARGET: ItemHookTarget = ItemHookTarget {
	name: "Item use",
	rva: 0x1fb05b0,
	prefix: ITEM_HOOK_PREFIX,
};

static ITEM_HOOK: OnceLock<GenericDetour<CopyFn>> = OnceLock::new();

type CopyFn = unsafe extern "C" fn(usize);

unsafe extern "C" fn copy_hook(p1: usize) {
	ITEM_HOOK.get().unwrap_unchecked().call(p1);

	let dst = *((p1 + 0x50) as *const usize);
	let size = *((p1 + 0x48) as *const usize);
	if dst == 0 || size < 32 {
		return;
	}
	let words = dst as *mut u32;
	// offset 7 bytes + 20 bits = 244 bits = virtual input index for UseItem
	if poll_use_quick_item() {
		*words.add(7) |= 0b00000000_00010000_00000000_00000000;
	}
}

const ITEM_HOOK_FN: CopyFn = copy_hook;

fn module_base() -> usize {
	unsafe { GetModuleHandleW(std::ptr::null()) as usize }
}

pub fn install_io_hooks() {
	unsafe {
		let base = module_base();

		install_input_hooks(base);
		install_item_hook(base);
	}
}

unsafe fn install_item_hook(base: usize) {
	if ITEM_HOOK.get().is_some() {
		return;
	}

	let addr = base + ITEM_HOOK_TARGET.rva;

	let bytes = std::slice::from_raw_parts(addr as *const u8, ITEM_HOOK_TARGET.prefix.len());
	if bytes != ITEM_HOOK_TARGET.prefix {
		let expected: String = ITEM_HOOK_TARGET.prefix.iter().map(|b| format!("{b:#02x}")).collect();
		let found: String = bytes.iter().map(|b| format!("{b:#02x}")).collect();
		tracing::warn!(
			"Couldn't hook {}: prologue mismatch at rva {:#x}: Expected 0x{expected}, found 0x{found}",
			ITEM_HOOK_TARGET.name, ITEM_HOOK_TARGET.rva,
		);
		return;
	}

	let target_fn: CopyFn = mem::transmute::<usize, CopyFn>(addr);
	let detour = match GenericDetour::<CopyFn>::new(target_fn, ITEM_HOOK_FN) {
		Ok(detour) => detour,
		Err(err) => {
			tracing::warn!("Couldn't hook {}: create failed: {err}", ITEM_HOOK_TARGET.name);
			return;
		}
	};

	let _ = ITEM_HOOK.set(detour);
	match ITEM_HOOK.get().unwrap().enable() {
		Ok(()) => tracing::info!("{} hooked", ITEM_HOOK_TARGET.name),
		Err(e) => tracing::warn!("{}: enable failed: {e}", ITEM_HOOK_TARGET.name),
	}
}

unsafe fn install_input_hooks(base: usize) {
	for (i, target) in INPUT_HOOK_TARGETS.iter().enumerate() {
		if INPUT_HOOKS[i].get().is_some() {
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
		let detour = match GenericDetour::<GenericHookFn>::new(target_fn, INPUT_HOOK_FNS[i]) {
			Ok(detour) => detour,
			Err(err) => {
				tracing::warn!("Couldn't hook {}: create failed: {err}", target.name);
				continue;
			}
		};

		let _ = INPUT_HOOKS[i].set(detour);

		match INPUT_HOOKS[i].get().unwrap().enable() {
			Ok(()) => tracing::info!("{} hooked", target.name),
			Err(e) => tracing::warn!("{}: enable failed: {e}", target.name),
		}
	}
}

pub fn uninstall_io_hooks() {
	unsafe {
		for h in INPUT_HOOKS.iter().filter_map(|h| h.get()) {
			let _ = h.disable();
		}
	}
}