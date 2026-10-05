use crate::debugging::{add_to_screen_debug, is_debugging};
use crate::keyboard;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

pub enum Listener {
	Pressed {
		keys: Box<[i32]>,
	},
	Released {
		keys: Box<[i32]>,
	},
	Held {
		keys: Box<[i32]>,
	},
}

impl Listener {
	pub fn keys(&self) -> &[i32] {
		match self {
			Listener::Pressed { keys }
			| Listener::Released { keys }
			| Listener::Held { keys } => keys
		}
	}
}

#[derive(Clone, Debug)]
pub struct ListenerStatus(Arc<AtomicBool>);

impl ListenerStatus {
	pub fn is_active(&self) -> bool {
		self.0.load(Ordering::Relaxed)
	}
}

lazy_static!(
	static ref PREV_TICK: Arc<Mutex<HashSet<i32>>> = Arc::new(Mutex::new(HashSet::new()));
	static ref LISTENERS: Arc<Mutex<Vec<(Listener, ListenerStatus)>>> = Arc::new(Mutex::new(Vec::new()));
);

pub fn tick_listeners() {
	let mut prev_tick = PREV_TICK.lock().expect("Couldn't acquire PREV_TICK");
	let listeners = LISTENERS.lock().expect("Couldn't acquire LISTENERS");

	let mut next_tick = HashSet::new();
	for (listener, status) in listeners.iter() {
		let keys = listener.keys();

		let all_keys_previously_pressed = keys.iter().all(|key| prev_tick.contains(key));
		let mut all_keys_currently_pressed = true;
		for &key in keys {
			if keyboard::is_pressed(key) {
				next_tick.insert(key);
			} else {
				all_keys_currently_pressed = false;
			}
		}

		if is_debugging() {
			add_to_screen_debug(format!("Listener status: {all_keys_previously_pressed}, {all_keys_currently_pressed}"));
		}
		if matches!(
			(listener, all_keys_previously_pressed, all_keys_currently_pressed),
			(Listener::Pressed{..}, false, true) |
			(Listener::Released{..}, true, false) |
			(Listener::Held{..}, _, true)
		) {
			tracing::info!("Listener activated");
			status.0.store(true, Ordering::Relaxed);
		} else {
			status.0.store(false, Ordering::Relaxed);
		}
	}

	*prev_tick = next_tick;
}

pub fn setup_listener(listener: Listener) -> ListenerStatus {
	let mut listeners = LISTENERS.lock().expect("Couldn't acquire LISTENERS");
	let status = ListenerStatus(Arc::new(AtomicBool::new(false)));
	listeners.push((listener, status.clone()));
	status
}