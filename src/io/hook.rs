use std::mem;
use retour::GenericDetour;
use std::sync::OnceLock;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::debugging::add_to_screen_debug;
use crate::io::bindings::{get_item_bindings, get_spell_bindings};
use crate::settings::Settings;

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
const TARGETS: [Target; NUM_HOOKS] = [
	Target {
		name: "Switch Spell request",
		rva: 0x759120,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Spell),
		controller_suppression_length: 0.0,
	},
	Target {
		name: "Switch Item request",
		rva: 0x7590B0,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Item),
		controller_suppression_length: 0.0,
	},
	Target {
		name: "Switch Spell repeat",
		rva: 0x759680,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Spell),
		controller_suppression_length: 0.5,
	},
	Target {
		name: "Switch Item repeat",
		rva: 0x7593D0,
		prefix: INPUT_CHECK_PREFIX,
		ids: Ok(SuppressionType::Item),
		controller_suppression_length: 0.5,
	},
	Target {
		name: "Switch Spell / Item hold",
		rva: 0x759270,
		prefix: SWITCH_HOLD_PREFIX,
		ids: Err(&[
			(13, SuppressionType::Spell),
			(14, SuppressionType::Item),
		]),
		controller_suppression_length: 0.0,
	},
];
static HOOKS: [OnceLock<GenericDetour<GenericHookFn>>; NUM_HOOKS] = [const { OnceLock::new() }; NUM_HOOKS];

unsafe extern "C" fn generic_hook<const I: usize>(a: usize, b: usize, c: usize, d: usize) -> usize {
	let settings = Settings::read_or_default();
	let items_key_ingame = settings.quick_items_key.to_lowercase() == "ingame";
	let spells_key_ingame = settings.spells_key.to_lowercase() == "ingame";

	let target = &TARGETS[I];

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
			add_to_screen_debug(format!("{items_pressed:?}"));
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
			add_to_screen_debug(format!("{spells_pressed:?}"));
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