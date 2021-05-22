use std::path::PathBuf;

use anyhow::*;
use bytemuck::{Pod, Zeroable};
use image::ImageFormat;
use wgpu::*;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::asset_loading::texture2::load_spritesheet_from_file;
use crate::render::camera::{Camera, PIXEL_RESOLUTION};
use crate::render::display::FrameBuilder;
use crate::render::vert::VertexSprite;
use crate::render::sprite::{Spritesheets, SpriteMap};
use std::sync::Arc;

struct Shaders {
	pub main_vs: ShaderModule,
	pub fullscreenquad_vs: ShaderModule,

	pub main_fs: ShaderModule,
	pub output_fs: ShaderModule,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Uniforms {
	camera_transform: [[f32; 4]; 4],
}

impl Uniforms {
	fn new() -> Self {
		use cgmath::SquareMatrix;
		Self {
			camera_transform: cgmath::Matrix4::identity().into(),
		}
	}

	fn update_camera_transform(&mut self, camera: &Camera) {
		self.camera_transform = camera.get_sprite_matrix().into();
	}
}

pub struct Renderer {
	window: Window,
	surface: Surface,
	device: Device,
	queue: Queue,
	swapchain_desc: SwapChainDescriptor,
	swapchain: SwapChain,
	size: PhysicalSize<u32>,

	shaders: Shaders,
	spritesheets: Spritesheets,

	pipeline_main: RenderPipeline,
	bind_group_output: BindGroup,
	pipeline_output: RenderPipeline,

	intermediate_texture: Texture,
	intermediate_texture_view: TextureView,

	uniforms: Uniforms,
	uniform_buffer: Buffer,
	uniform_bind_group: BindGroup,
}

impl Renderer {
	fn load_shaders(device: &Device) -> Shaders {
		// weird `Unexpected varying type` errors during loading
		let main_vs = device.create_shader_module(&include_spirv!(concat!(env!("OUT_DIR"), "/shaders/main.vert.spv")));
		let fullscreenquad_vs = device.create_shader_module(&include_spirv!(concat!(env!("OUT_DIR"), "/shaders/fullscreenquad.vert.spv")));

		// no errors
		let main_fs = device.create_shader_module(&include_spirv!(concat!(env!("OUT_DIR"), "/shaders/main.frag.spv")));
		let output_fs = device.create_shader_module(&include_spirv!(concat!(env!("OUT_DIR"), "/shaders/output.frag.spv")));

		Shaders {
			main_vs,
			fullscreenquad_vs,

			main_fs,
			output_fs,
		}
	}

	fn load_spritesheets(device: &Device, queue: &Queue) -> Result<Spritesheets> {
		let spritesheet_main = PathBuf::from(concat!(env!("OUT_DIR"), "/textures/spritesheet"));
		let spritesheet_main = load_spritesheet_from_file(
			spritesheet_main.with_extension("png"),
			spritesheet_main.with_extension("ron"),
			Some(ImageFormat::Png),
			TextureFormat::Rgba8UnormSrgb,
			device,
			queue,
			TextureUsage::SAMPLED | TextureUsage::COPY_DST)?;
		let mut spritesheets = Vec::new();
		spritesheets.push(spritesheet_main);
		Ok(Spritesheets::init(spritesheets))
	}

	// TODO handle errors nicely instead of unwrapping
	pub async fn new(events_loop: &EventLoop<()>) -> Self {
		let window = WindowBuilder::new().with_title("aaaaa").build(&events_loop).unwrap();
		let size = window.inner_size();

		let instance = Instance::new(BackendBit::PRIMARY);
		let surface = unsafe { instance.create_surface(&window) };
		let adapter = instance.request_adapter(
			&wgpu::RequestAdapterOptions {
				power_preference: PowerPreference::default(),
				compatible_surface: Some(&surface),
			},
		).await.unwrap();
		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				features: Features::empty(),
				limits: Limits::default(),
				label: None,
			},
			None,
		).await.unwrap();
		// TODO check format is what we want
		let format = adapter.get_swap_chain_preferred_format(&surface).unwrap();
		let swapchain_desc = wgpu::SwapChainDescriptor {
			usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
			format,
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo,
		};
		let swapchain = device.create_swap_chain(&surface, &swapchain_desc);

		let shaders = Renderer::load_shaders(&device);

		let spritesheets = Renderer::load_spritesheets(&device, &queue).unwrap();

		let uniforms = Uniforms::new();

		let uniform_buffer = device.create_buffer_init(
			&util::BufferInitDescriptor {
				label: Some("Uniform buffer"),
				contents: bytemuck::cast_slice(&[uniforms]),
				usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
			}
		);

		let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: ShaderStage::VERTEX,
					ty: BindingType::Buffer {
						ty: BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}
			],
			label: Some("Uniform bind group layout"),
		});

		let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
			layout: &uniform_bind_group_layout,
			entries: &[
				BindGroupEntry {
					binding: 0,
					resource: uniform_buffer.as_entire_binding(),
				}
			],
			label: Some("Uniform bind group"),
		});

		let pipeline_main_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Main render pipeline layout"),
				bind_group_layouts: &[
					&uniform_bind_group_layout,
				],
				push_constant_ranges: &[],
			});

		let pipeline_main = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render pipeline main"),
			layout: Some(&pipeline_main_layout),
			vertex: VertexState {
				module: &shaders.main_vs,
				entry_point: "main",
				buffers: &[
					VertexSprite::desc()
				],
			},
			fragment: Some(FragmentState {
				module: &shaders.main_fs,
				entry_point: "main",
				targets: &[ColorTargetState {
					format: TextureFormat::Rgba16Float,
					blend: Some(BlendState::REPLACE),
					write_mask: ColorWrite::ALL,
				}],
			}),
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: FrontFace::Ccw,
				polygon_mode: PolygonMode::Fill,
				clamp_depth: false,
				conservative: false,
				cull_mode: None
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
		});

		let intermediate_texture = device.create_texture(
			&TextureDescriptor {
				label: Some("Intermediate texture"),
				size: Extent3d {
					width: PIXEL_RESOLUTION[0],
					height: PIXEL_RESOLUTION[1],
					depth_or_array_layers: 1,
				},
				mip_level_count: 1,
				sample_count: 1,
				dimension: TextureDimension::D2,
				format: TextureFormat::Rgba16Float,
				usage: TextureUsage::SAMPLED | TextureUsage::RENDER_ATTACHMENT,
			}
		);

		let intermediate_texture_view = intermediate_texture.create_view(&TextureViewDescriptor::default());
		let intermediate_texture_sampler = device.create_sampler(&SamplerDescriptor {
			label: Some("Texture sampler"),
			address_mode_u: AddressMode::ClampToEdge,
			address_mode_v: AddressMode::ClampToEdge,
			address_mode_w: AddressMode::ClampToEdge,
			mag_filter: FilterMode::Nearest,
			min_filter: FilterMode::Nearest,
			mipmap_filter: FilterMode::Nearest,
			..Default::default()
		});

		let bind_group_output_layout = device.create_bind_group_layout(
			&BindGroupLayoutDescriptor {
				entries: &[
					BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: BindingType::Texture {
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
							sample_type: wgpu::TextureSampleType::Float { filterable: false },
						},
						count: None,
					},
					BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: BindingType::Sampler {
							comparison: false,
							filtering: false,
						},
						count: None,
					},
				],
				label: Some("Texture bind group layout"),
			}
		);

		let bind_group_output = device.create_bind_group(
			&BindGroupDescriptor {
				layout: &bind_group_output_layout,
				entries: &[
					BindGroupEntry {
						binding: 0,
						resource: wgpu::BindingResource::TextureView(&intermediate_texture_view),
					},
					BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::Sampler(&intermediate_texture_sampler),
					}
				],
				label: Some("Texture bind group"),
			}
		);

		let pipeline_output_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Main render pipeline layout"),
				bind_group_layouts: &[&bind_group_output_layout],
				push_constant_ranges: &[],
			});

		let pipeline_output = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render pipeline output"),
			layout: Some(&pipeline_output_layout),
			vertex: VertexState {
				module: &shaders.fullscreenquad_vs,
				entry_point: "main",
				buffers: &[],
			},
			fragment: Some(FragmentState {
				module: &shaders.output_fs,
				entry_point: "main",
				targets: &[ColorTargetState {
					format: swapchain_desc.format,
					blend: Some(BlendState::REPLACE),
					write_mask: ColorWrite::ALL,
				}],
			}),
			primitive: PrimitiveState {
				topology: PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: FrontFace::Ccw,
				polygon_mode: PolygonMode::Fill,
				clamp_depth: false,
				conservative: false,
				cull_mode: None
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
		});

		Self {
			window,
			surface,
			device,
			queue,
			swapchain_desc,
			swapchain,
			size,
			pipeline_main,
			bind_group_output,
			pipeline_output,
			shaders,
			spritesheets,
			intermediate_texture,
			intermediate_texture_view,
			uniforms,
			uniform_buffer,
			uniform_bind_group,
		}
	}

	pub fn get_sprite_map(&self) -> Arc<SpriteMap> {
		self.spritesheets.get_sprite_map()
	}

	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		// TODO format this nicely
		// TODO actually use env_logger?
		println!("resized {:?}", new_size);
		self.size = new_size;
		self.swapchain_desc.width = new_size.width;
		self.swapchain_desc.height = new_size.height;
		self.swapchain = self.device.create_swap_chain(&self.surface, &self.swapchain_desc);
	}

	pub fn draw_frame(&mut self, mut frame: FrameBuilder, camera: &Camera) -> Result<(), SwapChainError> {
		let swapchain_frame = self.swapchain.get_current_frame()?.output;

		self.uniforms.update_camera_transform(camera);

		let mut command_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
			label: Some("Command encoder")
		});

		let (vert, ind) = frame.get_sprite_renderer().get_buffers();
		let vert_buf = self.device.create_buffer_init(
			&util::BufferInitDescriptor {
				label: Some("Vertex buffer"),
				contents: bytemuck::cast_slice(vert.as_slice()),
				usage: BufferUsage::VERTEX
			}
		);
		let ind_buf = self.device.create_buffer_init(
			&util::BufferInitDescriptor {
				label: Some("Index buffer"),
				contents: bytemuck::cast_slice(ind.as_slice()),
				usage: BufferUsage::INDEX
			}
		);
		let num_indices = ind.len() as u32;

		// TODO staging buffer instead?
		self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));

		{
			let mut render_pass_main = command_encoder.begin_render_pass(&RenderPassDescriptor {
				label: Some("Render pass main"),
				depth_stencil_attachment: None,
				color_attachments: &[
					RenderPassColorAttachment {
						ops: Operations {
							load: LoadOp::Clear(Color {
								r: 0.0,
								g: 1.0,
								b: 0.0,
								a: 0.0
							}),
							store: true
						},
						view: &self.intermediate_texture_view,
						resolve_target: None,
					}
				]
			});
			render_pass_main.set_pipeline(&self.pipeline_main);
			render_pass_main.set_bind_group(0, &self.uniform_bind_group, &[]);
			render_pass_main.set_vertex_buffer(0, vert_buf.slice(..));
			render_pass_main.set_index_buffer(ind_buf.slice(..), IndexFormat::Uint32);
			render_pass_main.draw_indexed(0..num_indices, 0, 0..1);
		}
		{
			let mut render_pass_output = command_encoder.begin_render_pass(&RenderPassDescriptor {
				label: Some("Render pass output"),
				depth_stencil_attachment: None,
				color_attachments: &[
					RenderPassColorAttachment {
						ops: Operations {
							load: LoadOp::Clear(Color {
								r: 0.0,
								g: 0.0,
								b: 0.8,
								a: 0.0
							}),
							store: true
						},
						view: &swapchain_frame.view,
						resolve_target: None,
					}
				]
			});

			render_pass_output.set_pipeline(&self.pipeline_output);
			render_pass_output.set_bind_group(0, &self.bind_group_output, &[]);
			render_pass_output.draw(0..3, 0..1);
		}

		self.queue.submit(std::iter::once(command_encoder.finish()));

		Ok(())
	}
}
