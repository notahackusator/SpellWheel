use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use gamepads::{Button, Gamepads};

pub struct GamepadState {
    gamepads: Gamepads,
    pressed: HashMap<Button, Instant>,
    right_stick: RightStick,
    cached_data: GamepadData,
}

pub type Pressed = HashMap<Button, Duration>;
pub type Released = HashSet<Button>;
pub type RightStick = [f32; 2];
pub type GamepadData = (RightStick, Pressed, Released);

impl GamepadState {
    pub fn new() -> Self {
        Self {
            gamepads: Gamepads::new(),
            pressed: Default::default(),
            right_stick: Default::default(),
            cached_data: Default::default(),
        }
    }

    pub fn update(&mut self) {
        self.gamepads.poll();

        let now = Instant::now();
        let mut still_pressed = HashSet::new();

        self.right_stick = [0.0; 2];
        for gamepad in self.gamepads.all() {
            let right_stick = gamepad.right_stick().into();
            if right_stick != [0.0; 2] {
                self.right_stick = right_stick;
            }
            for button in gamepad.all_currently_pressed() {
                still_pressed.insert(button);
            }
        }

        let mut released = HashSet::new();
        for (button, _) in &self.pressed {
            if !still_pressed.contains(button) {
                released.insert(*button);
            }
        }

        // Update self.pressed
        self.pressed.retain(|button, _start| still_pressed.contains(button));

        // Retain only keys not yet in pressed
        still_pressed.retain(|button| !self.pressed.contains_key(button));
        for button in still_pressed {
            self.pressed.insert(button, now);
        }

        let mut pressed = HashMap::new();
        for (button, start) in &self.pressed {
            pressed.insert(*button, start.elapsed());
        }

        self.cached_data = (self.right_stick, pressed, released);
    }
    
    pub fn get_data(&self) -> GamepadData {
        self.cached_data.clone()
    }
}