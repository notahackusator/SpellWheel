use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::xinput_hook::get_xinput_gamepad_state;
use crate::PROGRAM_START;
use std::time::{Duration, Instant};
use lazy_static::lazy_static;
use windows::Win32::UI::Input::XboxController::*;

pub type GamepadButtons = XINPUT_GAMEPAD_BUTTON_FLAGS;

lazy_static!(
    pub static ref BUTTONS: HashMap<&'static str, Result<GamepadButtons, bool>> = HashMap::from_iter([
        ("UP", Ok(XINPUT_GAMEPAD_DPAD_UP)),
        ("DOWN", Ok(XINPUT_GAMEPAD_DPAD_DOWN)),
        ("LEFT", Ok(XINPUT_GAMEPAD_DPAD_LEFT)),
        ("RIGHT", Ok(XINPUT_GAMEPAD_DPAD_RIGHT)),
        ("A", Ok(XINPUT_GAMEPAD_A)),
        ("B", Ok(XINPUT_GAMEPAD_B)),
        ("X", Ok(XINPUT_GAMEPAD_X)),
        ("Y", Ok(XINPUT_GAMEPAD_Y)),
        ("R1", Ok(XINPUT_GAMEPAD_RIGHT_SHOULDER)),
        ("R2", Err(true)),
        ("R3", Ok(XINPUT_GAMEPAD_RIGHT_THUMB)),
        ("L1", Ok(XINPUT_GAMEPAD_LEFT_SHOULDER)),
        ("L2", Err(false)),
        ("L3", Ok(XINPUT_GAMEPAD_LEFT_THUMB)),
    ]);
);

#[derive(Clone, Debug)]
pub struct GamepadState {
    pub pressed: [Instant; 16],
    pub r2: Instant,
    pub l2: Instant,
    pub right_stick: RightStick,
}

pub type RightStick = [f32; 2];

impl GamepadState {
    pub fn new() -> Self {
        Self {
            pressed: [*PROGRAM_START; 16],
            r2: *PROGRAM_START,
            l2: *PROGRAM_START,
            right_stick: Default::default(),
        }
    }

    pub fn pressed_duration(&self, button: Result<GamepadButtons, bool>) -> Option<Duration> {
        match button {
            Ok(pressed_button) => {
                let start = self.pressed[pressed_button.0.trailing_zeros() as usize];

                if start == *PROGRAM_START {
                    return None;
                }
                Some(start.elapsed())
            }
            Err(is_r2) => {
                let start = if is_r2 {
                    self.r2
                } else {
                    self.l2
                };

                if start == *PROGRAM_START {
                    return None;
                }
                Some(start.elapsed())
            }
        }
    }

    pub fn update(&mut self) {
        let gamepad = get_xinput_gamepad_state();

        let now = Instant::now();

        let div = i16::MAX as f32;
        self.right_stick = [gamepad.sThumbRX as f32 / div, gamepad.sThumbRY as f32 / div];

        let currently_pressed = gamepad.wButtons.0;
        let r2_currently_pressed = gamepad.bRightTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD.0 as u8;
        let l2_currently_pressed = gamepad.bLeftTrigger > XINPUT_GAMEPAD_TRIGGER_THRESHOLD.0 as u8;

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

        if !r2_currently_pressed {
            self.r2 = *PROGRAM_START;
        } else if self.r2 == *PROGRAM_START {
            self.r2 = now;
        }

        if !l2_currently_pressed {
            self.l2 = *PROGRAM_START;
        } else if self.l2 == *PROGRAM_START {
            self.l2 = now;
        }

        if is_debugging() {
            add_to_screen_debug("Gamepad data:".to_string());
            add_to_screen_debug(format!(" Right stick: {:?}", self.right_stick));
            add_to_screen_debug(format!(" Pressed: {:?}", self.pressed));
        }
    }
}

lazy_static!(
    static ref GAMEPAD_STATE: OnceLock<Arc<Mutex<GamepadState>>> = OnceLock::new();
);

pub fn set_gamepad_state(gamepad_state: Arc<Mutex<GamepadState>>) -> Result<(), Arc<Mutex<GamepadState>>> {
    GAMEPAD_STATE.set(gamepad_state)
}

pub fn update_gamepad_state() {
    match GAMEPAD_STATE.get() {
        Some(gamepad_state) => gamepad_state.lock().unwrap().update(),
        None => tracing::error!("update_gamepad_data called before GAMEPAD_STATE was initialized"),
    }
}

pub fn gamepad_state() -> GamepadState {
    match GAMEPAD_STATE.get() {
        Some(gamepad_state) => gamepad_state.lock().unwrap().clone(),
        None => {
            tracing::error!("gamepad_data called before GAMEPAD_STATE was initialized");
            GamepadState::new()
        }
    }
}