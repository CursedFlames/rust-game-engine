use std::collections::HashMap;

use serde::Deserialize;
use slab::Slab;
use wgpu::{Texture, TextureView, TextureViewDescriptor};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
pub struct SpriteMetadata {
	// x y w h
	/// x y w h of the (possibly trimmed) sprite
	pub frame: [u32; 4],
	pub rotated: bool,
	pub trimmed: bool,

	/// position of the trimmed frame within the original sprite
	pub original_position: [u32; 2],
	/// size of the sprite before trimming
	pub original_size: [u32; 2],
}

struct TextureWrapper {
	// TODO getters and `new` instead of pub
	pub texture: Texture,
	pub default_view: TextureView,
}

pub struct Spritesheet {
	texture: TextureWrapper,
	sprite_metadata: Slab<SpriteMetadata>,
}

impl Spritesheet {
	pub fn create(texture: Texture, view: Option<TextureView>, metadata: HashMap<String, SpriteMetadata>)
			-> (Self, HashMap<String, usize>) {
		let view = view.unwrap_or_else(|| texture.create_view(&TextureViewDescriptor::default()));
		let texture = TextureWrapper {
			texture,
			default_view: view
		};
		let mut indices = HashMap::new();
		let mut sprite_metadata = Slab::with_capacity(metadata.len());
		for (id, meta) in metadata.into_iter() {
			let key = sprite_metadata.insert(meta);
			// TODO use try_insert when it's stabilized
			let previous_value = indices.insert(id, key);
			debug_assert!(previous_value.is_none());
		}
		(
			Self {
				texture,
				sprite_metadata,
			},
			indices
		)
	}

	fn get_texture(&self) -> &TextureWrapper {
		&self.texture
	}

	fn get_sprite_metadata(&self, position: usize) -> Option<&SpriteMetadata> {
		self.sprite_metadata.get(position)
	}
}


pub struct SpriteRef {
	pub spritesheet_index: usize,
	pub sprite_index: usize,
}

pub type SpriteMap = HashMap<String, SpriteRef>;

pub struct Spritesheets {
	spritesheets: Slab<Spritesheet>,
	/// Wrapped in an Arc as we need to access it from both game code and the renderer
	///
	/// Kinda gross but whatever
	sprite_map: Arc<SpriteMap>,
}

impl Spritesheets {
	pub fn init(spritesheets: Vec<(Spritesheet, HashMap<String, usize>)>) -> Self {
		let mut slab = Slab::with_capacity(spritesheets.len());
		let mut sprite_map = HashMap::new();
		for (spritesheet, indices) in spritesheets.into_iter() {
			let index = slab.insert(spritesheet);
			for (key, inner_index) in indices {
				// TODO check for duplicates?
				sprite_map.insert(
					key,
					SpriteRef {
						spritesheet_index: index,
						sprite_index: inner_index,
					}
				);
			}
		}
		Self {
			spritesheets: slab,
			sprite_map: Arc::new(sprite_map),
		}
	}

	pub fn get_sprite_map(&self) -> Arc<SpriteMap> {
		self.sprite_map.clone()
	}
}