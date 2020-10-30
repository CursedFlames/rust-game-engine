use vulkano::buffer::cpu_access::WriteLock;
use crate::render::vert::Vertex3d;
use vulkano::buffer::CpuAccessibleBuffer;
use crate::game::Pos;

pub type DisplayElementComponent = Box<dyn DisplayElement + Send + Sync>;

pub trait DisplayElement {
	fn draw(&self, renderer: &mut SpriteRenderer, pos: &Pos) -> ();
}

pub struct DisplayElementSquare {

}

impl DisplayElement for DisplayElementSquare {
	fn draw(&self, renderer: &mut SpriteRenderer, pos: &Pos) {
		renderer.draw_test_square(pos.x, pos.y);
	}
}

/// Stores information needed to render a given frame probably idk
pub struct FrameBuilder {
	sprite_renderer: SpriteRenderer,
	time: f32,
}

impl FrameBuilder {
	pub fn new(time: f32) -> Self {
		Self {
			sprite_renderer: SpriteRenderer::new(),
			time
		}
	}

	pub fn get_time(&self) -> f32 { self.time }

	pub fn get_sprite_renderer(&mut self) -> &mut SpriteRenderer { &mut self.sprite_renderer }
}

// SpriteCollector or smth might be a better name? It's instantiated every frame...
pub struct SpriteRenderer {
	vertices: Vec<Vertex3d>,
	indices: Vec<u32>,
}

impl SpriteRenderer {
	pub fn new() -> Self {
		Self {
			vertices: Vec::new(),
			indices: Vec::new(),
		}
	}

	/// Draw an 8x8 square. For testing until actual rendering stuff is implemented.
	pub fn draw_test_square(&mut self, x: i32, y: i32) {
		let x = x as f32;
		let y = y as f32;
		let offset = self.vertices.len() as u32;
		self.vertices.extend(vec![
			Vertex3d {position: [x, y, 0.0]},
			Vertex3d {position: [x+8.0, y, 0.0]},
			Vertex3d {position: [x, y+8.0, 0.0]},
			Vertex3d {position: [x+8.0, y+8.0, 0.0]},
		].into_iter());
		// 0 1 2 2 1 3
		self.indices.extend([
			offset,
			offset+1,
			offset+2,
			offset+2,
			offset+1,
			offset+3,
		].iter());
	}

	pub fn get_buffers(&self) -> (&Vec<Vertex3d>, &Vec<u32>) {
		(&self.vertices, &self.indices)
	}
}