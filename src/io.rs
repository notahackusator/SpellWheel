use std::mem;
use crate::debugging::{add_to_screen_debug, is_debugging, run_every, run_once};
use crate::gamepad::{BUTTONS, gamepad_state};
use crate::keyboard::KEYS;
use crate::rendering::wheel_renderer::WheelType;
use crate::settings::Settings;
use crate::{in_menus, keyboard};
use hudhook::windows::Win32::System::Threading::GetCurrentProcessId;
use hudhook::windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CAPITAL, VK_TAB};
use hudhook::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use lazy_static::lazy_static;
use std::sync::{LockResult, Mutex, MutexGuard, OnceLock};
use std::time::Duration;
use eldenring::cs::{CSInGamePad, UserInputKey};
use eldenring::fd4::FD4PadManager;
use fromsoftware_shared::FromStatic;
use retour::GenericDetour;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_0;
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_BUTTON_FLAGS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::keyboard::listener::{static_listener, Listener};

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
		if static_listener(
			"bomboclat".to_string(),
			|| Listener::Released {
				keys: Box::new([VK_0.0 as i32])
			}
		).is_active() {
			let key_assign = unsafe { pad.key_assign.as_ref() }; // if private, make it pub in your fork
			let ka = std::ptr::from_ref(key_assign).cast::<usize>();
			let input_mapper = unsafe { *ka.add(2) } as *const usize; // FD4BaseKeyAssign.input_mapper at +0x10

			unsafe {
				use dbg_hook::*;
				// log_vtable("key_assign", ka);
				// log_vtable("input_mapper", input_mapper);
				let inner = *input_mapper.add(1) as *const usize; // input_mapper + 8
				// log_vtable("input_mapper.inner", inner);

				let set = inner;
				let begin = unsafe { *set.add(2) } as *const usize; // +0x10
				let end = unsafe { *set.add(3) } as *const usize;   // +0x18
				let bytes = (end as usize).wrapping_sub(begin as usize);
				tracing::info!("mapper set: begin={begin:p} end={end:p} bytes={bytes:#x}");
				// for i in 0..((bytes / 8).min(8)) {
				// 	let e = unsafe { *begin.add(i) } as *const usize;
				// 	if e.is_null() {
				// 		tracing::info!("mapper[{i}]: null");
				// 		continue;
				// 	}
				// 	unsafe {
				// 		log_vtable(&format!("mapper[{i}]"), e);
				// 		log_qwords(&format!("mapper[{i}] raw"), e, 8);
				// 	}
				// }

				let m = unsafe { *begin } as *const usize;                    // mapper[0]
				let vbegin = unsafe { *m.add(3) } as *const usize;            // +0x18
				let vend   = unsafe { *m.add(4) } as *const usize;            // +0x20
				let n = ((vend as usize).wrapping_sub(vbegin as usize) / 8).min(32);
				tracing::info!("MapperImpl vec A: begin={vbegin:p} end={vend:p} n={n}");

				// for i in 0..n {
				// 	let e = unsafe { *vbegin.add(i) } as *const usize;
				// 	if !unsafe { readable(e.cast(), 0x40) } {
				// 		tracing::info!("A[{i}] = {e:p} (null or not readable)");
				// 		continue;
				// 	}
				// 	unsafe {
				// 		log_vtable(&format!("A[{i}]"), e);
				// 		log_qwords(&format!("A[{i}] raw"), e, 8);
				// 	}
				// }
				// unsafe { log_qwords("MapperImpl raw", m, 16) }; // also shows container B
				//
				// let base = GetModuleHandleW(std::ptr::null()) as usize;
				// if readable(m.add(15).cast(), 24) {
				// 	let (bb, be) = (*m.add(7) as *const usize,  *m.add(8) as *const usize);
				// 	let (cb, ce) = (*m.add(11) as *const usize, *m.add(12) as *const usize);
				// 	let (db, de) = (*m.add(15) as *const usize, *m.add(16) as *const usize);
				// 	dump_vec("B", bb, be, base);
				// 	dump_vec("C", cb, ce, base);
				// 	dump_vec("D", db, de, base); // D is a guess: it may not be a vector
				// }

				let bb = *m.add(7);
				let be = *m.add(8);
				// log_group_counts("B", bb as *const usize, be as *const usize);
				let pairs = collect_pairs(bb as *const usize, be as *const usize);

				if let Some(idx) = pad.input_type_group.as_ref().find(&UserInputKey::SwitchSpell)
					.and_then(|itg| pad.key_assign.as_ref().get_virtual_input_index(itg.mapped_input_list[0])) {
					for (_, code) in pairs.iter().filter(|(p_idx, _)| *p_idx == idx) {
						tracing::info!("decoded = {:?}", decode(*code));
					}
					// let ssk = pad.input_type_group.as_ref().find(&UserInputKey::SwitchSpell)
					// 	.and_then(|itg| pad.key_assign.as_ref().get_virtual_input_index(itg.mapped_input_list[0]))
					// 	.and_then(|idx| pairs.iter().find(|(p_idx, _)| p_idx == &idx))
					// 	.map(|(_, code)| *code);
					// tracing::info!("ssk={ssk:?}");
					// if let Some(ssk) = ssk {
					// 	tracing::info!("decode={:?}", decode(ssk));
					// }
				}
			}
		}
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
		// PREV_BUTTON_SPELLS.lock(), settings.spells_button.clone(), XINPUT_GAMEPAD_DPAD_UP,
		PREV_KEY_SPELLS.lock(), settings.spells_key.clone(), VK_TAB,
	);
	let quick_items = (
		WheelType::QuickItems,
		// PREV_BUTTON_QUICK_ITEMS.lock(), settings.quick_items_button.clone(), XINPUT_GAMEPAD_DPAD_DOWN,
		PREV_KEY_QUICK_ITEMS.lock(), settings.quick_items_key.clone(), VK_CAPITAL,
	);
	for (
		wheel_type,
		// prev_button, button, raw_button_default,
		prev_key, key, raw_key_default,
	) in [spells, quick_items] {
		// if let Some(value) = is_wheel_open_button_pressed(&settings, wheel_type, prev_button, button, raw_button_default) {
		// 	return value;
		// }

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

pub mod dbg_hook {
	//! Temporary probe: find out which object answers the HUD's action queries.
	//!
	//! Hooks FUN_14075e2d0 (RVA 0x75E2D0), the body of the lambda the switch-check
	//! wrappers build. Per Ghidra it does:
	//!
	//!     X = *(pad + 0x10);
	//!     return X->vtable[+0x28](X, *(int*)(lambda + 8), lambda + 0xc);
	//!
	//! This logs, once per distinct (action id, X's vtable), which vtable answers
	//! each action. Then in Ghidra: Go To (G) `0x140000000 + <vtable rva>` to read
	//! the class name, and open the function at `<slot5 rva>`.
	//!
	//! Read-only: it never changes arguments or the result. Remove it when done.
	//!
	//! Before using it: in Ghidra, select the first few instructions of
	//! FUN_14075e2d0, right click > Copy Special > Byte String, and paste the
	//! bytes into PROLOGUE below (about 8-10 bytes). It refuses to hook if empty
	//! or if the bytes don't match.

	use std::{collections::HashSet, sync::{Mutex, OnceLock}};

	use retour::GenericDetour;
	use windows::Win32::System::Memory::{VirtualQuery, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_NOACCESS, PAGE_GUARD};
	use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VSC_TO_VK_EX};
	use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

	type LambdaCall = unsafe extern "C" fn(lambda: *const u8, pad: *const u8) -> usize;

	const RVA: usize = 0x75E2D0;
	const PROLOGUE: &[u8] = &[0x44, 0x8b, 0x49, 0x08, 0x4c, 0x8b, 0xc1, 0x48, 0x8b];

	static HOOK: OnceLock<GenericDetour<LambdaCall>> = OnceLock::new();
	static SEEN: Mutex<Option<HashSet<(i32, usize)>>> = Mutex::new(None);

	fn module_base() -> usize {
		unsafe { GetModuleHandleW(std::ptr::null()) as usize }
	}

	pub unsafe fn log_qwords(label: &str, p: *const usize, count: usize) {
		for i in 0..count {
			tracing::info!("{label}[{i}] = {:#x}", *p.add(i));
		}
	}

	pub unsafe fn dump_vec(label: &str, begin: *const usize, end: *const usize, base: usize) {
		if begin.is_null() || end < begin { return; }
		let n = ((end as usize - begin as usize) / 8).min(256);
		tracing::info!("{label}: n={n}");
		for i in 0..n {
			let raw = *begin.add(i);
			let e = raw as *const usize;
			if readable(e.cast(), 0x20) {
				tracing::info!(
                "{label}[{i}] ptr={raw:#x} vt=base+{:#x} q1={:#x} q2={:#x} q3={:#x}",
                (*e).wrapping_sub(base), *e.add(1), *e.add(2), *e.add(3)
            );
			} else {
				tracing::info!("{label}[{i}] raw={raw:#x} (not a readable pointer)");
			}
		}
	}

	pub unsafe fn readable(p: *const u8, len: usize) -> bool {
		if p.is_null() { return false; }
		let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
		let sz = size_of::<MEMORY_BASIC_INFORMATION>();
		if VirtualQuery(Some(p.cast()), &mut mbi, sz) == 0 { return false; }
		let end = mbi.BaseAddress as usize + mbi.RegionSize;
		mbi.State == MEM_COMMIT
			&& mbi.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0) == 0
			&& p as usize + len <= end
	}

	pub unsafe fn is_readable(p: *const u8, len: usize) -> bool {
		if p.is_null() {
			return false;
		}
		let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
		let sz = size_of::<MEMORY_BASIC_INFORMATION>();
		if VirtualQuery(Some(p.cast()), &mut mbi, sz) == 0 {
			return false;
		}
		let end = mbi.BaseAddress as usize + mbi.RegionSize;
		mbi.State == MEM_COMMIT
			&& mbi.Protect.0 & (PAGE_NOACCESS.0 | PAGE_GUARD.0) == 0
			&& p as usize + len <= end
	}

	/// Read (a, b) from every element of a pointer vector [begin, end).
	/// Skips elements that aren't readable.
	pub unsafe fn collect_pairs(begin: *const usize, end: *const usize) -> Vec<(i32, i32)> {
		let mut out = Vec::new();
		if begin.is_null() || end < begin {
			return out;
		}
		let n = ((end as usize - begin as usize) / 8).min(1024);
		for i in 0..n {
			let e = *begin.add(i) as *const i32;
			if is_readable(e.cast(), 0x10) {
				out.push((*e.add(2), *e.add(3))); // +0x08 and +0x0C
			}
		}
		out
	}

	/// Log the pairs sorted, one per line, with no pointers in them, so two
	/// snapshots (before / after a rebind) can be compared with any text diff tool.
	pub unsafe fn log_pairs(label: &str, begin: *const usize, end: *const usize) {
		let mut pairs = collect_pairs(begin, end);
		pairs.sort_unstable();
		tracing::info!("{label}: {} pairs", pairs.len());
		for (a, b) in pairs {
			tracing::info!("{label} pair {a} {b}");
		}
	}

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum PadButton {
		DpadUp, DpadDown, DpadLeft, DpadRight,
		Start, Back, L3, R3,
		LB, RB, A, B, X, Y,
	}

	const PAD_ORDER: [PadButton; 14] = [
		PadButton::DpadUp, PadButton::DpadDown, PadButton::DpadLeft, PadButton::DpadRight,
		PadButton::Start, PadButton::Back, PadButton::L3, PadButton::R3,
		PadButton::LB, PadButton::RB, PadButton::A, PadButton::B, PadButton::X, PadButton::Y,
	];

	// VK_UP, VK_PRIOR, VK_LEFT, VK_RIGHT, VK_END, VK_DOWN, VK_NEXT, VK_INSERT, VK_DELETE
	const NAV_VK: [u32; 9] = [0x26, 0x21, 0x25, 0x27, 0x23, 0x28, 0x22, 0x2D, 0x2E];

	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum Physical {
		/// 0 = left, 1 = right, 2 = middle (assumed), 3 = button 4 (seen).
		Mouse(u8),
		Pad(PadButton),
		/// `vk` is the Windows virtual-key code when it can be derived.
		Key { vk: Option<u32> },
		Unknown(i32),
	}

	pub fn decode(code: i32) -> Physical {
		match code {
			0..=7 => Physical::Mouse(code as u8),
			40..=53 => Physical::Pad(PAD_ORDER[(code - 40) as usize]),
			70..=189 => Physical::Key { vk: dik_to_vk((code - 69) as u32) },
			190..=198 => Physical::Key { vk: Some(NAV_VK[(code - 190) as usize]) },
			_ => Physical::Unknown(code),
		}
	}

	/// Everything bound to one virtual input index, decoded.
	pub fn bindings_for(pairs: &[(i32, i32)], virtual_index: i32) -> Vec<Physical> {
		physical_for(pairs, virtual_index).into_iter().map(decode).collect()
	}

	pub unsafe fn log_group_counts(label: &str, begin: *const usize, end: *const usize) {
		use std::collections::BTreeMap;

		let mut by_index: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
		for (a, b) in collect_pairs(begin, end) {
			by_index.entry(a).or_default().push(b);
		}

		let mut counts: BTreeMap<Vec<i32>, usize> = BTreeMap::new();
		for (_, mut codes) in by_index {
			codes.sort_unstable();
			*counts.entry(codes).or_default() += 1;
		}
		for (codes, n) in counts {
			tracing::info!("{label} group {codes:?} x{n}");
		}
	}

	/// All physical codes bound to one virtual input index.
	pub fn physical_for(pairs: &[(i32, i32)], virtual_index: i32) -> Vec<i32> {
		pairs
			.iter()
			.filter(|(a, _)| *a == virtual_index)
			.map(|(_, b)| *b)
			.collect()
	}

	/// Convert a DirectInput-style scancode to a Windows virtual-key code.
	/// DirectInput sets 0x80 on extended keys (arrows, right Ctrl/Alt, numpad
	/// Enter, ...); MapVirtualKey wants an 0xE0 prefix for those.
	pub fn dik_to_vk(dik: u32) -> Option<u32> {
		let sc = if dik & 0x80 != 0 { 0xE000 | (dik & 0x7F) } else { dik };
		let vk = unsafe { MapVirtualKeyW(sc, MAPVK_VSC_TO_VK_EX) };
		(vk != 0).then_some(vk)
	}

	pub(crate) unsafe fn log_vtable(label: &str, obj: *const usize) {
		if obj.is_null() {
			tracing::info!("{label}: null");
			return;
		}
		let base = GetModuleHandleW(std::ptr::null()) as usize;
		let vtable = *obj; // the first qword of a C++ object is its vtable pointer
		let rva = vtable.wrapping_sub(base);
		tracing::info!("{label}: obj={obj:p} vtable=base+{rva:#x} (Ghidra: {:#x})", 0x1_4000_0000 + rva);
	}

	unsafe extern "C" fn detour(lambda: *const u8, pad: *const u8) -> usize {
		if !lambda.is_null() && !pad.is_null() {
			let id = unsafe { *(lambda.add(8) as *const i32) };
			let x = unsafe { *(pad.add(0x10) as *const *const usize) };
			if !x.is_null() {
				let vtable = unsafe { *x };
				let slot5 = unsafe { *((vtable + 0x28) as *const usize) };
				let base = module_base();

				// try_lock: never block the game thread on our logging.
				if let Ok(mut seen) = SEEN.try_lock() {
					if seen.get_or_insert_with(HashSet::new).insert((id, vtable)) {
						tracing::info!(
                        "action id {id}: X={x:p} vtable=base+{:#x} slot5=base+{:#x}",
                        vtable.wrapping_sub(base),
                        slot5.wrapping_sub(base),
                    );
					}
				}
			}
		}
		unsafe { HOOK.get().unwrap().call(lambda, pad) }
	}

	pub unsafe fn install() -> bool {
		if PROLOGUE.is_empty() {
			tracing::warn!("probe: PROLOGUE is empty, paste the bytes from Ghidra first");
			return false;
		}

		let addr = module_base() + RVA;
		let bytes = unsafe { std::slice::from_raw_parts(addr as *const u8, PROLOGUE.len()) };
		if bytes != PROLOGUE {
			tracing::warn!("probe: prologue mismatch at rva {RVA:#x}, skipping");
			tracing::warn!("  expected: {PROLOGUE:?}");
			tracing::warn!("  found:    {bytes:?}");
			return false;
		}

		let target: LambdaCall = unsafe { std::mem::transmute::<usize, LambdaCall>(addr) };
		let d = match unsafe { GenericDetour::<LambdaCall>::new(target, detour) } {
			Ok(d) => d,
			Err(e) => {
				tracing::warn!("probe: create failed: {e}");
				return false;
			}
		};

		// Publish before enabling, same as the main hooks.
		let _ = HOOK.set(d);
		unsafe { HOOK.get().unwrap().enable().is_ok() }
	}

	pub unsafe fn uninstall() {
		if let Some(h) = HOOK.get() {
			let _ = unsafe { h.disable() };
		}
	}
}