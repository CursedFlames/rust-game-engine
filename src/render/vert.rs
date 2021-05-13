use bytemuck::{Pod, Zeroable};
use wgpu::{VertexBufferLayout, VertexAttribute, VertexFormat::*, BufferAddress};

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex2d {
	pub position: [f32; 2],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct Vertex3d {
	pub position: [f32; 3],
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable)]
pub struct VertexSprite {
	pub position: [f32; 3], // 12 bytes
	pub uv: [f32; 2], // 12 + 8 = 20 bytes
	// pub tint: [u8; 4], // 20 + 4 = 24 bytes
	// TODO other properties like emissivity, light occlusion and such
	// will probably want to compress UVs so that the entire vert fits in 32 bytes in that case
}

impl Vertex2d {
}

impl Vertex3d {
}

impl VertexSprite {
	// TODO how can we use `vertex_attr_array` without getting lifetime issues?
	pub fn desc() -> VertexBufferLayout<'static> {
		VertexBufferLayout {
			array_stride: std::mem::size_of::<VertexSprite>() as wgpu::BufferAddress,
			step_mode: wgpu::InputStepMode::Vertex,
			attributes: &[
				VertexAttribute { shader_location: 0, format: Float32x3, offset: 0 },
				VertexAttribute { shader_location: 1, format: Float32x3, offset: std::mem::size_of::<[f32; 2]>() as BufferAddress },
			],
		}
	}
}
