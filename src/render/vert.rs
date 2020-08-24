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
