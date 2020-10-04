use cgmath::Vector2;

pub const PIXEL_RESOLUTION: [u32; 2] = [320, 180];
pub const CAMERA_CENTER_POS: [f64; 2] = [
	160.0,
	90.0,
];
// This stuff was for sub-pixel movement, using full pixel movement rn so don't need it
/*pub const PIXEL_MARGIN: [u32; 2] = [2, 2];
pub const PIXEL_OFFSET: [u32; 2] = [1, 1];
pub const PIXEL_FULL_RESOLUTION: [u32; 2] = [
	PIXEL_RESOLUTION[0] + PIXEL_MARGIN[0],
	PIXEL_RESOLUTION[1] + PIXEL_MARGIN[1],
];
// TODO camera_width_height as float form of pixel_full_resolution?
//      and the actually visible width/height too
pub const CAMERA_CENTER_POS: [f64; 2] = [
	PIXEL_FULL_RESOLUTION[0] as f64 / 2.0,
	PIXEL_FULL_RESOLUTION[1] as f64 / 2.0,
];*/

pub struct Camera {
	pub pos: cgmath::Vector2<f64>,
}

impl Camera {
	pub fn new() -> Self {
		Camera {
			pos: Vector2::new(CAMERA_CENTER_POS[0], CAMERA_CENTER_POS[1])
		}
	}

	fn get_game_pos_f64(&self) -> Vector2<f64> {
		Vector2::new(self.pos.x.floor(), self.pos.y.floor())
	}

	fn get_game_pos(&self) -> Vector2<i32> {
		Vector2::new(self.pos.x.floor() as i32, self.pos.y.floor() as i32)
	}

	pub fn get_sprite_matrix(&self) -> cgmath::Matrix4<f32> {
		let pos = self.get_game_pos_f64();
		let pixel_offset = cgmath::Matrix4::from_translation(
			cgmath::Vector3::new(
				(CAMERA_CENTER_POS[0]-pos.x) as f32,
				(CAMERA_CENTER_POS[1]-pos.y) as f32,
				0.0));
		// 2.0 instead of 1.0 as x,y need to span from -1 to 1
		let scale = cgmath::Matrix4::from_nonuniform_scale(
			2.0/PIXEL_RESOLUTION[0] as f32,// 2.0/PIXEL_FULL_RESOLUTION[0] as f64,
			2.0/PIXEL_RESOLUTION[1] as f32,// 2.0/PIXEL_FULL_RESOLUTION[1] as f64,
			1.0);
		// translate from (0, 2) to (-1, 1) range
		let final_translate = cgmath::Matrix4::from_translation(
			cgmath::Vector3::new(-1.0, -1.0, 0.0));
		// flip axes (our code uses Y+ up, Vulkan seems to use Y+ down
		let axis_flip = cgmath::Matrix4::from_nonuniform_scale(1.0, -1.0, 1.0);

		axis_flip * final_translate * scale * pixel_offset
	}
}
