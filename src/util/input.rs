use winit::event::VirtualKeyCode;
use std::collections::HashMap;

pub struct InputMap {
	// Held, down, up
	key_states: HashMap<VirtualKeyCode, (bool, bool, bool)>,
	buffered_inputs: Vec<KeyInput>,
}

enum KeyInput {
	KeyDown(VirtualKeyCode),
	KeyUp(VirtualKeyCode),
}

use KeyInput::*;

impl InputMap {
	pub fn new() -> Self {
		InputMap {
			key_states: HashMap::new(),
			buffered_inputs: Vec::new(),
		}
	}

	pub fn buffer_keydown(&mut self, key: VirtualKeyCode) {
		self.buffered_inputs.push(KeyDown(key));
	}

	pub fn buffer_keyup(&mut self, key: VirtualKeyCode) {
		self.buffered_inputs.push(KeyUp(key));
	}

	fn apply_input(&mut self, input: KeyInput) {
		let key = match input {
			KeyDown(k) => k,
			KeyUp(k) => k
		};
		let entry = self.key_states.entry(key).or_insert((false, false, false));
		match input {
			KeyDown(..) => {
				// Don't give a "key just pressed" if the key was already down
				if !entry.0 {
					entry.0 = true;
					entry.1 = true;
				}
			},
			KeyUp(..) => {
				// Similarly here
				if entry.0 {
					entry.0 = false;
					entry.2 = true;
				}
			}
		}
	}

	/// Should be called at the beginning of a tick.
	/// Consumes all buffered inputs and updates the keyboard state.
	pub fn begin_tick(&mut self) {
		// Need to collect here to prevent multiple mutable borrows
		for key in self.buffered_inputs.drain(..).collect::<Vec<_>>() {
			self.apply_input(key);
		}
	}

	/// Should be called at the end of a tick.
	/// Clears all keydowns/keyups.
	pub fn end_tick(&mut self) {
		for entry in self.key_states.iter_mut() {
			(entry.1).1 = false;
			(entry.1).2 = false;
		}
	}

	/// Should be removed once we switch to action-based inputs
	pub fn get_key_pressed(&self, key: VirtualKeyCode) -> bool {
		self.key_states.get(&key).unwrap_or(&(false, false, false)).0
	}
}