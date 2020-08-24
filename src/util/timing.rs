use std::time::Duration;

pub struct TickTiming {
	tick_duration: f64,
	partial_ticks: f64,
}

impl TickTiming {
	pub fn new(tick_duration: f64) -> Self {
		TickTiming {
			tick_duration,
			partial_ticks: 0.0,
		}
	}

	pub fn add_delta(&mut self, delta: Duration) {
		self.partial_ticks += delta.as_secs_f64()/self.tick_duration;

		// TODO extract constant for max built up tick time
		if self.partial_ticks > 5.0 {
			self.partial_ticks = 5.0;
		}
	}

	/// If one or more ticks should be done, decrements the internal counter by one and returns true
	pub fn try_consume_tick(&mut self) -> bool {
		if self.partial_ticks >= 1.0 {
			self.partial_ticks -= 1.0;
			return true;
		}
		false
	}
}
