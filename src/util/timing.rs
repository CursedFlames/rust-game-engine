use std::time::{Duration, Instant};
use crate::render::renderer2::Renderer;
use crate::game::Game;

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

	pub fn get_partial_ticks(&self) -> f64 {
		self.partial_ticks
	}
}

// I named this SpicyTiming while testing alternative timings, but it's funny so I'm keeping it
pub struct SpicyTiming {
	last_updated_time: Instant,
	target_duration: Duration,
	tick_duration: Duration,
	max_elapsed: Duration,
	// TODO does carrying this over from previous ticks improve things or make it worse? hard to tell
	total_elapsed: Duration,
}

impl SpicyTiming {
	pub fn new() -> Self {
		Self {
			last_updated_time: Instant::now(),
			target_duration: Duration::from_secs_f64(1.0/60.0),
			tick_duration: Duration::from_secs_f64(1.0/60.0),
			max_elapsed: Duration::from_secs_f64(0.5),
			// TODO use Duration::ZERO when it's stabilized
			total_elapsed: Duration::from_secs(0),
		}
	}

	pub fn update(&mut self, game: &mut Game, renderer: &mut Renderer) {
		// This loop shouldn't actually be necessary, assuming sleep is reliable
		// (could just copy the logic above the break to be after the sleep as well)
		// keeping it anyway for now, since it's marginally more readable imo
		loop {
			let now = Instant::now();
			self.total_elapsed += now.duration_since(self.last_updated_time);
			self.last_updated_time = now;
			if self.total_elapsed > self.target_duration {
				break;
			}
			spin_sleep::sleep(self.target_duration - self.total_elapsed);
		}
		if self.total_elapsed > self.max_elapsed {
			self.total_elapsed = self.max_elapsed;
		}
		while self.total_elapsed >= self.tick_duration {
			self.total_elapsed -= self.tick_duration;
			// TODO tick count
			game.tick(0);
		}
		// TODO tick count and timing values
		game.draw_frame(renderer, 0, 0.0, 0.0);
	}
}
