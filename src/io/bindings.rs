use crate::gamepad::{GamepadButtons, GamepadState, gamepad_state};
use crate::keyboard;
use crate::settings::Settings;
use bitfield::bitfield;
use eldenring::cs::{CSInGamePad, UserInputKey};
use lazy_static::lazy_static;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, RwLock};
use windows::Win32::UI::Input::KeyboardAndMouse::{MapVirtualKeyW, MAPVK_VSC_TO_VK_EX, VK_0};
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y};
use crate::debugging::run_once;
use crate::keyboard::listener::{static_listener, Listener};

lazy_static!(
	static ref SPELL_BINDINGS: Arc<RwLock<PhysicalKeys>> = Arc::new(RwLock::new(PhysicalKeys::none()));
	static ref ITEM_BINDINGS: Arc<RwLock<PhysicalKeys>> = Arc::new(RwLock::new(PhysicalKeys::none()));
);

pub fn set_spell_bindings(physical_keys: PhysicalKeys) {
	*SPELL_BINDINGS.write().expect("Couldn't acquire SPELL_BINDINGS") = physical_keys;
}

pub fn set_item_bindings(physical_keys: PhysicalKeys) {
	*ITEM_BINDINGS.write().expect("Couldn't acquire ITEM_BINDINGS") = physical_keys;
}

pub fn get_spell_bindings() -> PhysicalKeys {
	*SPELL_BINDINGS.read().expect("Couldn't acquire SPELL_BINDINGS")
}

pub fn get_item_bindings() -> PhysicalKeys {
	*ITEM_BINDINGS.read().expect("Couldn't acquire ITEM_BINDINGS")
}

pub struct Bindings {
	entries: Vec<BindingEntry>,
}

impl Bindings {
	pub fn get_input_bindings(&self, pad: &CSInGamePad, input: &UserInputKey) -> PhysicalKeys {
		unsafe {
			let Some(itg) = pad.input_type_group.as_ref().find(input) else {
				return PhysicalKeys::none();
			};
			let Some(idx) = pad.key_assign.as_ref().get_virtual_input_index(itg.mapped_input_list[0]) else {
				return PhysicalKeys::none();
			};
			self.entries.iter().filter(|entry| entry.for_virtual_index(idx)).collect()
		}
	}

	pub fn from_pad(pad: &CSInGamePad) -> Self {
		unsafe {
			let key_assign = pad.key_assign.as_ref();
			let key_assign_ptr = std::ptr::from_ref(key_assign).cast::<usize>();
			let input_mapper_ptr = *key_assign_ptr.add(2) as *const usize;

			let input_mapper_inner = *input_mapper_ptr.add(1) as *const usize;

			let begin = *input_mapper_inner.add(2) as *const usize;

			let binding_vectors = *begin as *const usize;

			let mut digital_entries = BindingEntry::from_binding_vec(
				*binding_vectors.add(7) as *const usize, *binding_vectors.add(8) as *const usize
			);
			let mut analog_entries = BindingEntry::from_binding_vec(
				*binding_vectors.add(15) as *const usize, *binding_vectors.add(16) as *const usize
			);
			let mut entries = digital_entries;
			entries.append(&mut analog_entries);

			Self {
				entries
			}
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PhysicalKeys {
	pub which_present: u8,
	pub mouse: u8,
	pub pad: Result<GamepadButtons, bool>,
	pub key: u32,
	pub unknown: i32,
}

bitfield! {
	#[derive(Clone, Copy, PartialEq, Eq)]
	pub struct PressedStatus(u8);
	pub mouse, set_mouse: 0;
	pub pad,   set_pad:   1;
	pub key,   set_key:   2;
}

impl PhysicalKeys {
	pub fn none() -> Self {
		Self {
			which_present: 0,
			mouse: 0,
			pad: Ok(GamepadButtons::default()),
			key: 0,
			unknown: 0,
		}
	}

	pub fn mouse(&mut self, mouse: u8) {
		self.which_present |= 0b0001;
		self.mouse = mouse;
	}

	pub fn pad(&mut self, pad: Result<GamepadButtons, bool>) {
		self.which_present |= 0b0010;
		self.pad = pad;
	}

	pub fn key(&mut self, key: u32) {
		self.which_present |= 0b0100;
		self.key = key;
	}

	pub fn unknown(&mut self, unknown: i32) {
		self.which_present |= 0b1000;
		self.unknown = unknown;
	}

	pub fn pressed_status(&self, min_pad_press_length: Option<f32>) -> PressedStatus {
		let mut ps = PressedStatus(0);
		if self.which_present & 0b001 != 0 {
			ps.set_mouse(keyboard::is_pressed(self.mouse as i32));
		}
		if self.which_present & 0b010 != 0 {
			let controller_delay = Settings::read_or_default().controller_wheel_open_delay;
			let [min, max] = if let Some(min_length) = min_pad_press_length {
				[min_length, controller_delay]
			} else {
				[controller_delay, f32::MAX]
			};
			if min == 0.0 && max != 0.0 {
				ps.set_pad(GamepadState::new_active_state().pressed_duration(self.pad).is_some());
			} else if let Some(length) = gamepad_state().pressed_duration(self.pad) {
				let length = length.as_secs_f32();
				ps.set_pad(min <= length && length <= max);
			}
		}
		if self.which_present & 0b100 != 0 {
			ps.set_key(keyboard::is_pressed(self.key as i32));
		}
		ps
	}
}

impl Debug for PhysicalKeys {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if self.which_present & 0b0001 != 0 {
			write!(f, "mouse={} ", self.mouse)?;
		}
		if self.which_present & 0b0010 != 0 {
			write!(f, "pad={:?} ", self.pad)?;
		}
		if self.which_present & 0b0100 != 0 {
			write!(f, "key={} ", self.key)?;
		}
		if self.which_present & 0b1000 != 0 {
			write!(f, "unknown={} ", self.unknown)?;
		}
		Ok(())
	}
}

impl<'a> FromIterator<&'a BindingEntry> for PhysicalKeys {
	fn from_iter<T: IntoIterator<Item=&'a BindingEntry>>(iter: T) -> Self {
		let mut this = Self::none();
		for entry in iter {
			match entry.decode() {
				PhysicalInput::Mouse(mouse) => this.mouse(mouse),
				PhysicalInput::PadTriggers(is_right) => this.pad(Err(is_right)),
				PhysicalInput::Pad(pad) => this.pad(Ok(pad)),
				PhysicalInput::Key(key) => this.key(key),
				PhysicalInput::Unknown(unknown) => this.unknown(unknown),
			}
		}
		this
	}
}

impl Debug for PressedStatus {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if self.mouse() {
			write!(f, "m")?;
		}
		if self.pad() {
			write!(f, "p")?;
		}
		if self.key() {
			write!(f, "k")?;
		}
		Ok(())
	}
}

pub struct BindingEntry {
	pub virtual_index: i32,
	pub code: i32,
}

impl BindingEntry {
	pub fn new(virtual_index: i32, code: i32) -> Self {
		Self {
			virtual_index,
			code,
		}
	}

	pub fn for_virtual_index(&self, virtual_index: i32) -> bool {
		self.virtual_index == virtual_index
	}

	pub fn decode(&self) -> PhysicalInput {
		match self.code {
			0..=7 => PhysicalInput::Mouse(self.code as u8),
			11..=12 => PhysicalInput::PadTriggers(self.code == 12),
			40..=53 => PhysicalInput::Pad(PAD_ORDER[(self.code - 40) as usize]),
			70..=189 => PhysicalInput::Key(di_to_vk((self.code - 69) as u32)),
			190..=198 => PhysicalInput::Key(NAV_VK[(self.code - 190) as usize]),
			_ => PhysicalInput::Unknown(self.code),
		}
	}

	pub unsafe fn from_binding_vec(vec_begin: *const usize, vec_end: *const usize) -> Vec<Self> {
		let mut out = Vec::new();
		if vec_begin.is_null() || vec_end < vec_begin {
			return out;
		}
		let n = (vec_end as usize - vec_begin as usize) / 8;
		for i in 0..n {
			let entry_ptr = *vec_begin.add(i) as *const i32;
			if !entry_ptr.is_null() {
				out.push(Self::new(*entry_ptr.add(2), *entry_ptr.add(3)));
			}
		}
		out
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalInput {
	Mouse(u8),
	PadTriggers(bool),
	Pad(GamepadButtons),
	Key(u32),
	Unknown(i32),
}

pub fn di_to_vk(di: u32) -> u32 {
	let sc = if di & 0x80 != 0 {
		0xE000 | (di & 0x7F)
	} else {
		di
	};
	unsafe { MapVirtualKeyW(sc, MAPVK_VSC_TO_VK_EX) }
}

// VK_UP, VK_PRIOR, VK_LEFT, VK_RIGHT, VK_END, VK_DOWN, VK_NEXT, VK_INSERT, VK_DELETE
const NAV_VK: [u32; 9] = [0x26, 0x21, 0x25, 0x27, 0x23, 0x28, 0x22, 0x2D, 0x2E];
const PAD_ORDER: [GamepadButtons; 14] = [
	XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT,
	XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_THUMB,
	XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_RIGHT_SHOULDER,
	XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
];