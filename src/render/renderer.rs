use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, BufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::{PipelineLayoutAbstract, DescriptorSet};
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPass, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, StorageImage, SwapchainImage};
use vulkano::image::Dimensions::Dim2d;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::memory::pool::{PotentialDedicatedAllocation, StdMemoryPoolAlloc};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::pipeline::shader::GraphicsEntryPointAbstract;
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};
use vulkano::swapchain::{self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use crate::render::vert;
use crate::render::vert::Vertex2d;
use crate::vulkutil;

pub const RESOLUTION: [u32; 2] = [320, 180];

// TODO figure out where to put shaders
//      apparently they don't rebuild when changed unless other things in the file are also changed?
//      irritating if true.
pub mod vs {
	vulkano_shaders::shader! {
		ty: "vertex",
		src: "\
#version 450
layout(location = 0) in vec2 position;

layout(location = 0) out vec2 fragTexCoord;

void main() {
	gl_Position = vec4(position, 0.0, 1.0);
	fragTexCoord = position;
}"
	}
}

pub mod fs_triangle {
	vulkano_shaders::shader! {
		ty: "fragment",
		src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform PushConstants {
	float time;
} pushConstants;

void main() {
	f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
}"
	}
}

pub mod fs_output {
	vulkano_shaders::shader! {
		ty: "fragment",
		src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

layout(push_constant) uniform PushConstants {
	float time;
} pushConstants;

layout(binding = 0) uniform sampler2D texSampler;

void main() {
	// f_color = vec4(sin(pushConstants.time/4.0), 0.25, 1.0, 1.0);
	f_color = texture(texSampler, fragTexCoord);
}"
	}
}

pub struct Renderer {
	// TODO these shouldn't all be pub
	instance: Arc<Instance>,

	pub surface: Arc<Surface<Window>>,

	// Rust *really* doesn't like it when we try to store the physical device directly.
	// Wind up with a bunch of weird lifetime issues that I don't understand. :shrug:
	// TODO Seta seemed to store the physical device directly; do it how he did it
	physical_device_index: usize,
	pub device: Arc<Device>,

	pub graphics_queue: Arc<Queue>,

	pub swapchain: Arc<Swapchain<Window>>,
	pub swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

	// This stuff should eventually get moved into its own struct
	pub intermediate_image: Arc<AttachmentImage>,
	pub sampler_simple_nearest: Arc<Sampler>,
	pub vertex_buffer_triangle: Arc<dyn BufferAccess + Send + Sync>,
	pub vertex_buffer_square: Arc<dyn BufferAccess + Send + Sync>,
	pub render_pass_main: Arc<dyn RenderPassAbstract + Send + Sync>,
	pub render_pass_output: Arc<dyn RenderPassAbstract + Send + Sync>,
	pub pipeline_main: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
	pub pipeline_output: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
	pub descriptor_set_output: Arc<dyn DescriptorSet + Send + Sync>,
	pub dynamic_state: DynamicState,
	pub framebuffer_main: Arc<dyn FramebufferAbstract + Send + Sync>,
	pub framebuffers_output: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,

	pub previous_frame_end: Option<Box<dyn GpuFuture>>,
	pub recreate_swapchain: bool,
}

impl Renderer {
	pub fn init(events_loop: &EventLoop<()>) -> Self {
		let instance = {
			let extensions = vulkano_win::required_extensions();
			Instance::new(None, &extensions, None)
				.expect("Failed to create Vulkan instance.")
		};
		let physical = vulkutil::select_physical_device(&instance);
		let physical_device_index = physical.index();

		let surface = Self::create_window(&instance, &events_loop);

		let (device, queue) = Self::get_device_and_queue(physical);

		let (swapchain, swapchain_images) =
			Self::create_swapchain(physical, &device, &surface, &queue);

		// Whole ton of stuff that needs to get ripped out into other functions and such

		let mut intermediate_image = AttachmentImage::with_usage(
			device.clone(),
			RESOLUTION,
			Format::R16G16B16A16Sfloat,
			ImageUsage {
				storage: true,
				color_attachment: true,
				sampled: true,
				..ImageUsage::none()
			}
		).expect("Failed to create intermediate image");

		let sampler_simple_nearest = Sampler::new(
			device.clone(),
			Filter::Nearest,
			Filter::Nearest,
			MipmapMode::Nearest,
			SamplerAddressMode::ClampToEdge,
			SamplerAddressMode::ClampToEdge,
			SamplerAddressMode::ClampToEdge,
			0.0,
			1.0,
			0.0,
			1.0
		).expect("Failed to create sampler");

		// TODO move all this buffer stuff somewhere more sensible

		let vertex_buffer_triangle = {
			CpuAccessibleBuffer::from_iter(
				device.clone(),
				// TODO pick actual BufferUsage
				BufferUsage::all(),
				false,
				[
					Vertex2d {position: [-0.5, -0.25]},
					Vertex2d {position: [0.0, 0.5]},
					Vertex2d {position: [0.25, -0.1]},
				].iter().cloned()
			).unwrap()
		};

		let vertex_buffer_square = {
			CpuAccessibleBuffer::from_iter(
				device.clone(),
				// TODO pick actual BufferUsage
				BufferUsage::all(),
				false,
				[
					// TODO do quads properly lmao
					Vertex2d {position: [-1.0, -1.0]},
					Vertex2d {position: [-1.0, 1.0]},
					Vertex2d {position: [1.0, -1.0]},
					Vertex2d {position: [1.0, 1.0]},
					Vertex2d {position: [-1.0, 1.0]},
					Vertex2d {position: [1.0, -1.0]},
				].iter().cloned()
			).unwrap()
		};

		// let fragment_uniform_buffer = CpuBufferPool::<fs_output::ty::unf_data>::new(device.clone(), BufferUsage::all());

		let vs = vs::Shader::load(device.clone()).unwrap();
		let fs_triangle = fs_triangle::Shader::load(device.clone()).unwrap();
		let fs_output = fs_output::Shader::load(device.clone()).unwrap();

		let render_pass_main: Arc<RenderPass<_>> = Arc::new(
			vulkano::single_pass_renderpass!(
			device.clone(),
			attachments: {
				color: {
					load: Clear,
					store: Store,
					format: Format::R16G16B16A16Sfloat,
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {}
			}
		).unwrap()
		);

		let render_pass_output: Arc<RenderPass<_>> = Arc::new(
			vulkano::single_pass_renderpass!(
			device.clone(),
			attachments: {
				color: {
					// attachment is cleared upon draw
					// TODO in future some of these can be replaced with DontCare
					//      will want to implement more first before doing that though.
					load: Clear,
					store: Store,
					format: swapchain.format(),
					// no idea what this does, but the vulkano example uses it
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {}
			}
		).unwrap()
		);

		let pipeline_main = Arc::new(
			GraphicsPipeline::start()
				.vertex_input_single_buffer::<Vertex2d>()
				.vertex_shader(vs.main_entry_point(), ())
				.triangle_list()
				.viewports(vec![Viewport {
					origin: [0.0, 0.0],
					dimensions: [RESOLUTION[0] as f32, RESOLUTION[1] as f32],
					depth_range: 0.0..1.0,
				}])
				.fragment_shader(fs_triangle.main_entry_point(), ())
				.render_pass(Subpass::from(render_pass_main.clone(), 0).unwrap())
				.build(device.clone())
				.unwrap()
		);

		let pipeline_output = Arc::new(
			GraphicsPipeline::start()
				.vertex_input_single_buffer::<Vertex2d>()
				.vertex_shader(vs.main_entry_point(), ())
				.triangle_list()
				.viewports_dynamic_scissors_irrelevant(1)
				.fragment_shader(fs_output.main_entry_point(), ())
				.render_pass(Subpass::from(render_pass_output.clone(), 0).unwrap())
				.build(device.clone())
				.unwrap()
		);

		let layout = pipeline_output.layout().descriptor_set_layout(0).expect("Failed to get set layout");

		// Doing this here is janky and will not work for changeable descriptor sets.
		let descriptor_set_output = Arc::new(
			PersistentDescriptorSet::start(layout.clone())
				.add_sampled_image(intermediate_image.clone(), sampler_simple_nearest.clone())
				.expect("Failed to add sampled image")
				.build()
				.expect("Failed to build descriptor set"),
		);

		// Need this for dynamically updating the viewport when resizing the window.
		let mut dynamic_state = DynamicState::none();

		let framebuffer_main = Arc::new(
			Framebuffer::start(render_pass_main.clone())
				.add(intermediate_image.clone())
				.unwrap()
				.build()
				.unwrap()
		);

		let mut framebuffers_output = Self::window_size_dependent_setup(
			&swapchain_images, render_pass_output.clone(), &mut dynamic_state);

		// I'm not clear on what exactly this does, but it sounds important for freeing memory that's no longer needed
		let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

		Self {
			instance,
			surface,
			physical_device_index, device,
			graphics_queue: queue,
			swapchain, swapchain_images,

			intermediate_image,
			sampler_simple_nearest,
			vertex_buffer_triangle,
			vertex_buffer_square,
			render_pass_main,
			render_pass_output,
			pipeline_main,
			pipeline_output,
			descriptor_set_output,
			dynamic_state,
			framebuffer_main,
			framebuffers_output,
			previous_frame_end,
			recreate_swapchain: false,
		}
	}

	fn create_window(instance: &Arc<Instance>, events_loop: &EventLoop<()>) -> Arc<Surface<Window>> {
		WindowBuilder::new()
			.with_title("Vulkan")
			.build_vk_surface(events_loop, instance.clone())
			.unwrap()
	}

	fn get_device_and_queue(physical: PhysicalDevice) -> (Arc<Device>, Arc<Queue>) {
		let queue_family = physical.queue_families()
			.find(|&q| q.supports_graphics())
			.expect("Failed to find a graphical queue family");

		println!("Selected queue family: {:?}", queue_family);

		let device_ext = DeviceExtensions {
			khr_swapchain: true,
			..DeviceExtensions::none()
		};
		let (device, mut queues) = {
			Device::new(physical, &Features::none(), &device_ext,
						[(queue_family, 0.5)].iter().cloned()).expect("Failed to create device")
		};

		// We only have one queue
		// TODO use multiple queues?
		let queue = queues.next().unwrap();
		(device, queue)
	}

	fn create_swapchain(physical: PhysicalDevice, device: &Arc<Device>, surface: &Arc<Surface<Window>>, queue: &Arc<Queue>)
						-> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
		let caps = surface.capabilities(physical).unwrap();
		// TODO we probably want to actually pick this properly?
		//      Seems to normally be opaque, but shouldn't rely on that.
		let alpha = caps.supported_composite_alpha.iter().next().unwrap();
		println!("Using alpha mode {:?}", alpha);
		// TODO formats?
		let format = caps.supported_formats[0].0;
		println!("Using format {:?}", format);

		let dimensions: [u32; 2] = surface.window().inner_size().into();

		Swapchain::new(
			device.clone(),
			surface.clone(),
			caps.min_image_count,
			format,
			dimensions,
			1,
			ImageUsage::color_attachment(),
			queue,
			SurfaceTransform::Identity,
			alpha,
			PresentMode::Fifo,
			FullscreenExclusive::Default,
			true,
			ColorSpace::SrgbNonLinear
		).unwrap()
	}

	fn window_size_dependent_setup(
		images: &[Arc<SwapchainImage<Window>>],
		render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
		dynamic_state: &mut DynamicState
	) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
		let dimensions = images[0].dimensions();

		let viewport = Viewport {
			origin: [0.0, 0.0],
			dimensions: [dimensions[0] as f32, dimensions[1] as f32],
			depth_range: 0.0..1.0,
		};
		dynamic_state.viewports = Some(vec![viewport]);

		images
			.iter()
			.map(|image| {
				Arc::new(
					Framebuffer::start(render_pass.clone())
						.add(image.clone())
						.unwrap()
						.build()
						.unwrap(),
				) as Arc<dyn FramebufferAbstract + Send + Sync>
			})
			.collect::<Vec<_>>()
	}

	pub fn draw_frame(&mut self, time_elapsed: f32) {
		// Free resources that are no longer needed? :shrug:
		self.previous_frame_end.as_mut().unwrap().cleanup_finished();

		if self.recreate_swapchain {
			let dimensions: [u32; 2] = self.surface.window().inner_size().into();
			let (new_swapchain, new_images) =
				match self.swapchain.recreate_with_dimensions(dimensions) {
					Ok(r) => r,
					// This tends to happen while the user is resizing the window, apparently
					Err(SwapchainCreationError::UnsupportedDimensions) => return,
					Err(e) => panic!("Failed to recreate swapchain: {:?}", e)
				};

			self.swapchain = new_swapchain;
			self.framebuffers_output = Self::window_size_dependent_setup(
				&new_images,
				self.render_pass_output.clone(),
				&mut self.dynamic_state,
			);
			self.recreate_swapchain = false;
		}

		let (image_num, suboptimal, acquire_future) =
			match swapchain::acquire_next_image(self.swapchain.clone(), None) {
				Ok(r) => r,
				Err(AcquireError::OutOfDate) => {
					self.recreate_swapchain = true;
					return;
				}
				Err(e) => panic!("Failed to acquire next image: {:?}", e)
			};

		if suboptimal {
			self.recreate_swapchain = true;
		}

		let push_constants = fs_output::ty::PushConstants {
			time: time_elapsed
		};

		let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];

		let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
			self.device.clone(),
			self.graphics_queue.family(),
		).unwrap();

		builder
			.begin_render_pass(self.framebuffer_main.clone(), false, clear_values.clone())
			.unwrap()
			.draw(
				self.pipeline_main.clone(),
				&DynamicState::none(),
				vec![self.vertex_buffer_triangle.clone()],
				(),
				push_constants
			)
			.unwrap()
			.end_render_pass()
			.unwrap();

		builder
			.begin_render_pass(self.framebuffers_output[image_num].clone(), false, clear_values)
			.unwrap()
			.draw(
				self.pipeline_output.clone(),
				&self.dynamic_state,
				vec![self.vertex_buffer_square.clone()],
				self.descriptor_set_output.clone(),
				push_constants
			)
			.unwrap()
			.end_render_pass()
			.unwrap();

		let command_buffer = builder.build().unwrap();

		let future = self.previous_frame_end
			.take()
			.unwrap()
			.join(acquire_future)
			.then_execute(self.graphics_queue.clone(), command_buffer)
			.unwrap()
			.then_swapchain_present(self.graphics_queue.clone(), self.swapchain.clone(), image_num)
			.then_signal_fence_and_flush();

		match future {
			Ok(future) => {
				self.previous_frame_end = Some(future.boxed());
			},
			Err(FlushError::OutOfDate) => {
				self.recreate_swapchain = true;
				self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
			},
			Err(e) => {
				println!("Failed to flush future: {:?}", e);
				self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
			}
		}
	}
}
