#[derive(Default, Debug, Clone)]
pub struct Vertex2d {
	pub position: [f32; 2],
}
vulkano::impl_vertex!(Vertex2d, position);

#[derive(Default, Debug, Clone)]
pub struct Vertex3d {
	pub position: [f32; 3],
}
vulkano::impl_vertex!(Vertex3d, position);

#[derive(Default, Debug, Clone)]
pub struct VertexSprite {
	pub position: [f32; 3], // 12 bytes
	pub uv: [f32; 2], // 12 + 8 = 20 bytes
	// pub tint: [u8; 4], // 20 + 4 = 24 bytes
	// TODO other properties like emissivity, light occlusion and such
	// will probably want to compress UVs so that the entire vert fits in 32 bytes in that case
}
vulkano::impl_vertex!(VertexSprite, position, uv);
