use std::thread;
use std::time::{Instant, Duration};

pub struct Timing {
	tick_duration: f64,

	partial_ticks: f64,
	last_frame_time: Option<Instant>,
	total_time: f64,
}

impl Timing {
	pub fn new() -> Self {
		Self {
			tick_duration: 1.0/60.0,
			partial_ticks: 0.0,
			last_frame_time: None,
			total_time: 0.0,
		}
	}

	/// Should be called during the MainEventsCleared window event, after any call to wait_for_next_frame
	/// Returns the time delta.
	pub fn on_update(&mut self) -> f64 {
		let delta = if let Some(instant) = self.last_frame_time {
			instant.elapsed().as_secs_f64()
		} else {
			0.0
		};

		self.total_time += delta;
		self.partial_ticks += delta/self.tick_duration;
		// TODO extract constant for max built up tick time
		if self.partial_ticks > 5.0 {
			self.partial_ticks = 5.0;
		}

		self.last_frame_time = Some(Instant::now());
		delta
	}

	pub fn get_total_time(&self) -> f64 { self.total_time }
	pub fn get_partial_ticks(&self) -> f64 { self.partial_ticks }

	pub fn wait_for_next_frame(&self) {
		// TODO wait modes
		//      sleep_with_buffer(amt)
		//      sleep_autoadjust_with_buffer(amt)
		//      no_wait
		// sleep for 1ms for now - TODO actual sleep length
		thread::sleep(Duration::new(0, 1000000));
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
