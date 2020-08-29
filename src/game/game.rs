use hecs::World;
use crate::render::display::{DisplayElementSquare, FrameBuilder, DisplayElement, DisplayElementComponent};
use crate::render::renderer::Renderer;

pub struct Pos {
	pub x: i32,
	pub y: i32,
}

struct Vel {
	pub vx: i32,
	pub vy: i32,
}

pub struct Game {
	level: World,
}

impl Game {
	pub fn new() -> Self {
		let mut level = World::new();
		level.spawn_batch(
			(0..10)
				.map(|i|
					(Pos {x: i*20, y: (i*70)%170},
						DisplayElementComponent(Box::new(DisplayElementSquare{})),
						Vel { vx: 1, vy: 1},
					)
				));
		Game {
			level,
		}
	}

	pub fn tick(&mut self, tick_count: u32) {
		if tick_count % 60 == 0 {
			println!("Game tick!");
		}
		let mut query = self.level.query::<(&mut Pos, &mut Vel)>();
		for (id, (pos, vel)) in query.iter() {
			pos.x += vel.vx;
			pos.y += vel.vy;
			if pos.x < 0 && vel.vx < 0 {
				vel.vx *= -1;
			}
			if pos.y < 0 && vel.vy < 0 {
				vel.vy *= -1;
			}
			if pos.x > 312 && vel.vx > 0 {
				vel.vx *= -1;
			}
			if pos.y > 172 && vel.vy > 0 {
				vel.vy *= -1;
			}
		}
	}

	pub fn draw_frame(&self, renderer: &mut Renderer, tick_count: u32, partial_ticks: f32, time: f32) {
		let mut frame = FrameBuilder::new(time);
		let sprite_renderer = frame.get_sprite_renderer();
		let mut query = self.level.query::<(&Pos, & DisplayElementComponent)>();
		for (id, (pos, display)) in query.iter() {
			display.0.draw(sprite_renderer, pos);
		}
		renderer.draw_frame(frame);
	}
}