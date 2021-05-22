use log::*;

use crate::game::Pos;
use crate::render::animation::AnimationFrame;
use crate::render::sprite::Spritesheet;
use crate::render::vert::VertexSprite;

pub type DisplayElementComponent = Box<dyn DisplayElement + Send + Sync>;

pub trait DisplayElement {
	fn draw(&self, renderer: &mut SpriteRenderer, pos: &Pos) -> ();
}

// TODO: animated sprites
pub struct DisplayElementFixedSprite {
	pub sprite: AnimationFrame,
}

impl DisplayElement for DisplayElementFixedSprite {
	fn draw(&self, renderer: &mut SpriteRenderer, pos: &Pos) {
		renderer.draw_sprite(self.sprite.offset([pos.x, pos.y]));
	}
}

/// Stores information needed to render a given frame probably idk
pub struct FrameBuilder {
	sprite_renderer: SpriteRenderer,
	time: f32,
}

impl FrameBuilder {
	// TODO do we want tick count + partial ticks as well?
	//      is time (roughly) equivalent to tick count + partial ticks? idk
	pub fn new(time: f32, spritesheet_count: usize) -> Self {
		Self {
			sprite_renderer: SpriteRenderer::new(spritesheet_count),
			time
		}
	}

	pub fn get_time(&self) -> f32 { self.time }

	pub fn get_sprite_renderer(&mut self) -> &mut SpriteRenderer { &mut self.sprite_renderer }
}

// TODO SpriteCollector or smth might be a better name? It's instantiated every frame...
pub struct SpriteRenderer {
	sprites: Vec<Vec<AnimationFrame>>,
}

impl SpriteRenderer {
	pub fn new(spritesheet_count: usize) -> Self {
		Self {
			sprites: vec![Vec::new(); spritesheet_count],
		}
	}

	pub fn draw_sprite(&mut self, sprite: AnimationFrame) {
		self.sprites[sprite.sprite.spritesheet_index].push(sprite);
	}

	pub fn get_buffers(&self, index: usize, spritesheet: &Spritesheet) -> Option<(Vec<VertexSprite>, Vec<u32>)> {
		let sprites = &self.sprites[index];
		if sprites.is_empty() {
			return None;
		}
		let mut vertices: Vec<VertexSprite> = Vec::with_capacity(sprites.len() * 4);
		let mut indices: Vec<u32> = Vec::with_capacity(sprites.len() * 6);
		for sprite in sprites.iter() {
			let size = spritesheet.size();
			let size = [size[0] as f32, size[1] as f32];
			if let Some(metadata) = spritesheet.get_sprite_metadata(sprite.sprite.sprite_index) {
				// TODO we could do this coordinate scaling on load, instead of for every sprite, every frame...
				let u1 = (metadata.frame[0] as f32)/size[0];
				let v1 = (metadata.frame[1] as f32)/size[1];
				let u2 = ((metadata.frame[0]+metadata.frame[2]) as f32)/size[0];
				let v2 = ((metadata.frame[1]+metadata.frame[3]) as f32)/size[1];
				let x1 = (sprite.offset[0] + metadata.original_position[0] as i32) as f32;
				let y1 = (sprite.offset[1] + metadata.original_position[1] as i32) as f32;
				// TODO what if we want to scale sprites? should AnimationFrame have a scale on it too?
				let x2 = x1 + metadata.frame[2] as f32;
				let y2 = y1 + metadata.frame[3] as f32;
				let offset = vertices.len() as u32;
				vertices.extend(vec![
					VertexSprite {position: [x1, y1, 0.0], uv: [u1, v2]},
					VertexSprite {position: [x2, y1, 0.0], uv: [u2, v2]},
					VertexSprite {position: [x1, y2, 0.0], uv: [u1, v1]},
					VertexSprite {position: [x2, y2, 0.0], uv: [u2, v1]},
				].into_iter());
				// 0 1 2 2 1 3
				indices.extend([
					offset,
					offset+1,
					offset+2,
					offset+2,
					offset+1,
					offset+3,
				].iter());
			} else {
				// TODO proper error handling?
				warn!("Failed to get sprite metadata for sprite {:?}", sprite.sprite);
			}
		}
		Some((vertices, indices))
	}
}