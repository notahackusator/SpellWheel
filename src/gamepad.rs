use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::xinput_hook::get_xinput_gamepad_state;
use crate::PROGRAM_START;
use std::time::Instant;
use windows::Win32::UI::Input::XboxController::XINPUT_GAMEPAD_BUTTON_FLAGS;

pub type GamepadButtons = XINPUT_GAMEPAD_BUTTON_FLAGS;

#[derive(Clone, Debug)]
pub struct GamepadState {
    pub pressed: [Instant; 16],
    pub right_stick: RightStick,
}

pub type RightStick = [f32; 2];

impl GamepadState {
    pub fn new() -> Self {
        Self {
            pressed: [*PROGRAM_START; 16],
            right_stick: Default::default(),
        }
    }

    pub fn update(&mut self) {
        let gamepad = get_xinput_gamepad_state();

        let now = Instant::now();

        let div = i16::MAX as f32;
        self.right_stick = [gamepad.sThumbRX as f32 / div, gamepad.sThumbRY as f32 / div];

        let currently_pressed = gamepad.wButtons.0;

        // Update self.pressed
        for i in 0..16 {
            // Remove released buttons
            if (currently_pressed >> i) % 2 == 0 {
                self.pressed[i] = *PROGRAM_START;
            }
            // Add pressed buttons
            else if self.pressed[i] == *PROGRAM_START {
                self.pressed[i] = now;
            }
        }

        if is_debugging() {
            add_to_screen_debug("Gamepad data:".to_string());
            add_to_screen_debug(format!(" Right stick: {:?}", self.right_stick));
            add_to_screen_debug(format!(" Pressed: {:?}", self.pressed));
        }
    }
}