use crate::render::sprite::SpriteRef;

// TODO add x/y scale as well?
#[derive(Copy, Clone, Debug)]
pub struct AnimationFrame {
	pub sprite: SpriteRef,
	pub offset: [i32; 2],
}

impl AnimationFrame {
	pub fn offset(&self, offset: [i32; 2]) -> Self {
		Self {
			offset: [self.offset[0]+ offset[0], self.offset[1] + offset[1]],
			..*self
		}
	}
}
