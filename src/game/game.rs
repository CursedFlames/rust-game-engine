use hecs::World;
use crate::render::display::{DisplayElementSquare, FrameBuilder, DisplayElement, DisplayElementComponent};
use crate::render::renderer::Renderer;
use crate::render::camera::Camera;
use crate::util::input::InputMap;
use winit::event::VirtualKeyCode;

use crate::game::physics;
use crate::game::physics::{PhysicsActorComponent, DebugPhysicsActor, PhysicsActor};

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
	pub camera: Camera, // TODO make this one non-public once we're doing inputs in a non-jank way
	pub input: InputMap, // TODO probably same for this and add methods on Game to pass through inputs?
}

impl Game {
	pub fn new() -> Self {
		let mut camera = Camera::new();
		let mut input = InputMap::new();
		let mut level = World::new();
		level.spawn_batch(
			(0..10)
				.map(|i|
					(
						Pos {x: i*20, y: (i*70)%170},
						DisplayElementComponent(Box::new(DisplayElementSquare{})),
						Vel { vx: 1, vy: 1},
					)
				));
		// level.spawn_batch(
		// 	(0..4)
		// 		.map(|i|
		// 			(
		// 				Pos {x: (i%2)*312, y: (i/2)*172},
		// 				DisplayElementComponent(Box::new(DisplayElementSquare{})),
		// 			)
		// 		));
		level.spawn((
				DisplayElementComponent(Box::new(DisplayElementSquare{})),
				PhysicsActorComponent(Box::new(DebugPhysicsActor{x:0,y:0})),
			));
		Game {
			level,
			camera,
			input,
		}
	}

	pub fn tick(&mut self, tick_count: u32) {
		if tick_count % 60 == 0 {
			println!("Game tick!");
		}
		self.input.begin_tick();

		// Temporary camera movement code
		let speed = if self.input.get_key_pressed(VirtualKeyCode::LShift) { 0.5 } else { 4.0 };
		let in_x = {
			let mut i = 0.0;
			if self.input.get_key_pressed(VirtualKeyCode::A) { i -= 1.0 };
			if self.input.get_key_pressed(VirtualKeyCode::D) { i += 1.0 };
			i
		};
		let in_y = {
			let mut i = 0.0;
			if self.input.get_key_pressed(VirtualKeyCode::S) { i -= 1.0 };
			if self.input.get_key_pressed(VirtualKeyCode::W) { i += 1.0 };
			i
		};
		self.camera.pos.x += in_x * speed;
		self.camera.pos.y += in_y * speed;

		physics::tick_physics(&mut self.level);

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

		self.input.end_tick();
	}

	pub fn draw_frame(&self, renderer: &mut Renderer, tick_count: u32, partial_ticks: f32, time: f32) {
		let mut frame = FrameBuilder::new(time);
		let sprite_renderer = frame.get_sprite_renderer();
		let mut query = self.level.query::<(&Pos, & DisplayElementComponent)>();
		for (id, (pos, display)) in query.iter() {
			display.0.draw(sprite_renderer, pos);
		}
		let mut query = self.level.query::<(&PhysicsActorComponent, &DisplayElementComponent)>();
		for (id, (pos, display)) in query.iter() {
			display.0.draw(sprite_renderer, &Pos {x: pos.0.x(), y: pos.0.y()});
		}
		renderer.draw_frame(frame, &self.camera);
	}
}