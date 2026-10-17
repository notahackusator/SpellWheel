use std::fmt::{Debug, Formatter};
use bitfield::bitfield;
use eldenring::cs::{CSInGamePad, UserInputKey};
use windows::Win32::UI::Input::KeyboardAndMouse::{MAPVK_VSC_TO_VK_EX, MapVirtualKeyW};
use windows::Win32::UI::Input::XboxController::{XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y};
use crate::gamepad::{gamepad_state, GamepadButtons};
use crate::keyboard;
use crate::settings::Settings;

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

			let bindings_vec = *begin as *const usize;

			let vec_begin = *bindings_vec.add(7);
			let vec_end = *bindings_vec.add(8);
			let bindings = BindingEntry::from_binding_vec(vec_begin as *const usize, vec_end as *const usize);

			Self {
				entries: bindings
			}
		}
	}
}

pub struct PhysicalKeys {
	pub which_present: u8,
	pub mouse: u8,
	pub pad: GamepadButtons,
	pub key: u32,
}

bitfield! {
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
			pad: GamepadButtons::default(),
			key: 0,
		}
	}

	pub fn mouse(&mut self, mouse: u8) {
		self.which_present |= 0b001;
		self.mouse = mouse;
	}

	pub fn pad(&mut self, pad: GamepadButtons) {
		self.which_present |= 0b010;
		self.pad = pad;
	}

	pub fn key(&mut self, key: u32) {
		self.which_present |= 0b100;
		self.key = key;
	}

	pub fn pressed_status(&self) -> PressedStatus {
		let mut ps = PressedStatus(0);
		if self.which_present & 0b001 != 0 {
			ps.set_mouse(keyboard::is_pressed(self.mouse as i32));
		}
		if self.which_present & 0b010 != 0 {
			ps.set_pad(
				gamepad_state().pressed_duration(Ok(self.pad))
					.is_some_and(
						|length| length.as_secs_f32() > Settings::read_or_default().controller_wheel_open_delay
					)
			);
		}
		if self.which_present & 0b100 != 0 {
			ps.set_key(keyboard::is_pressed(self.key as i32));
		}
		ps
	}
}

impl Debug for PhysicalKeys {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		if self.which_present & 0b001 != 0 {
			write!(f, "mouse={} ", self.mouse)?;
		}
		if self.which_present & 0b010 != 0 {
			write!(f, "pad={:?} ", self.pad)?;
		}
		if self.which_present & 0b100 != 0 {
			write!(f, "key={} ", self.key)?;
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
				PhysicalInput::Pad(pad) => this.pad(pad),
				PhysicalInput::Key(key) => this.key(key),
				PhysicalInput::Unknown(_) => {}
			}
		}
		this
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