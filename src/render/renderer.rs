use std::sync::Arc;

use vulkano::buffer::{BufferAccess, BufferUsage, CpuAccessibleBuffer, TypedBufferAccess, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, AutoCommandBuffer, CommandBufferExecFuture};
use vulkano::descriptor::{DescriptorSet, PipelineLayoutAbstract};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPass, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::pipeline::viewport::Viewport;
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};
use vulkano::swapchain::{self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain, SwapchainCreationError, PresentFuture, SwapchainAcquireFuture};
use vulkano::sync::{self, FlushError, GpuFuture, JoinFuture, FenceSignalFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::render::vert::{Vertex2d, Vertex3d};
use crate::render::display::FrameBuilder;
use vulkano::command_buffer::pool::standard::StandardCommandPoolAlloc;
use crate::render::camera::Camera;

pub const RESOLUTION: [u32; 2] = [320, 180];

fn select_physical_device(instance: &Arc<Instance>) -> PhysicalDevice {
	let devices: Vec<PhysicalDevice> = PhysicalDevice::enumerate(&instance).collect();

	println!("{} devices found:", devices.len());
	for dev in &devices {
		println!("{:?} ({}, type: {:?})", dev, dev.name(), dev.ty());
	}

	// TODO gracefully fail if no devices found - how?
	let physical = *devices.get(0).expect("Failed to find any available devices");

	println!("Selected device: {}", physical.name());

	physical
}

// Not sure if this is the best name for this struct but whatever.
// Contains all the various things used in the actual rendering process
// (as opposed to Renderer, which just has devices and queues and the swapchain and such)
struct RenderData {
	intermediate_image: Arc<AttachmentImage>,
	sampler_simple_nearest: Arc<Sampler>,
	vertex_buffer_pool: CpuBufferPool<Vertex3d>,
	index_buffer_pool: CpuBufferPool<u32>,
	vertex_buffer_square: Arc<dyn BufferAccess + Send + Sync>,
	render_pass_main: Arc<dyn RenderPassAbstract + Send + Sync>,
	render_pass_output: Arc<dyn RenderPassAbstract + Send + Sync>,
	pipeline_main: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
	pipeline_output: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
	descriptor_set_output: Arc<dyn DescriptorSet + Send + Sync>,
	dynamic_state: DynamicState,
	framebuffer_main: Arc<dyn FramebufferAbstract + Send + Sync>,
}

impl RenderData {
	fn init(device: &Arc<Device>, swapchain_format: Format) -> Self {
		let intermediate_image = AttachmentImage::with_usage(
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

		let vertex_buffer_pool_triangle = CpuBufferPool::new(device.clone(), BufferUsage::all());
		let index_buffer_pool_triangle = CpuBufferPool::new(device.clone(), BufferUsage::all());

		let vertex_buffer_square = {
			CpuAccessibleBuffer::from_iter(
				device.clone(),
				// TODO pick actual BufferUsage
				BufferUsage::all(),
				false,
				[
					// Full screen quad using an oversized triangle.
					// Slightly more efficient than two triangles, I think? :shrug:
					Vertex2d {position: [-1.0, -1.0]},
					Vertex2d {position: [-1.0, 3.0]},
					Vertex2d {position: [3.0, -1.0]},
				].iter().cloned()
			).unwrap()
		};

		// let fragment_uniform_buffer = CpuBufferPool::<fs_output::ty::unf_data>::new(device.clone(), BufferUsage::all());

		let vs = shaders::vs::Shader::load(device.clone()).unwrap();
		let vs_output = shaders::vs_output::Shader::load(device.clone()).unwrap();
		let fs_triangle = shaders::fs_triangle::Shader::load(device.clone()).unwrap();
		let fs_output = shaders::fs_output::Shader::load(device.clone()).unwrap();

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
					format: swapchain_format,
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
				.vertex_input_single_buffer::<Vertex3d>()
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
				.vertex_shader(vs_output.main_entry_point(), ())
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

		Self {
			intermediate_image,
			sampler_simple_nearest,
			vertex_buffer_pool: vertex_buffer_pool_triangle,
			index_buffer_pool: index_buffer_pool_triangle,
			vertex_buffer_square,
			render_pass_main,
			render_pass_output,
			pipeline_main,
			pipeline_output,
			descriptor_set_output,
			dynamic_state,
			framebuffer_main,
		}
	}
}

pub struct Renderer {
	instance: Arc<Instance>,

	surface: Arc<Surface<Window>>,

	// Rust *really* doesn't like it when we try to store the physical device directly.
	// Wind up with a bunch of weird lifetime issues that I don't understand. :shrug:
	// TODO Seta seemed to store the physical device directly; do it how he did it
	physical_device_index: usize,
	device: Arc<Device>,

	graphics_queue: Arc<Queue>,

	swapchain: Arc<Swapchain<Window>>,
	swapchain_images: Vec<Arc<SwapchainImage<Window>>>,

	data: RenderData,

	framebuffers_output: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,

	previous_frame_end: Option<Box<dyn GpuFuture>>,
	// TODO `on_resize` method instead of this - we'll need to handle other things like scaling anyway
	pub recreate_swapchain: bool,
}

impl Renderer {
	pub fn init(events_loop: &EventLoop<()>) -> Self {
		let instance = {
			let extensions = vulkano_win::required_extensions();
			Instance::new(None, &extensions, None)
				.expect("Failed to create Vulkan instance.")
		};
		let physical = select_physical_device(&instance);
		let physical_device_index = physical.index();

		let surface = Self::create_window(&instance, &events_loop);

		let (device, queue) = Self::get_device_and_queue(physical);

		let (swapchain, swapchain_images) =
			Self::create_swapchain(physical, &device, &surface, &queue);

		let mut render_data = RenderData::init(&device, swapchain.format());

		let framebuffers_output = Self::window_size_dependent_setup(
			&swapchain_images, render_data.render_pass_output.clone(), &mut render_data.dynamic_state);

		// I'm not clear on what exactly this does, but it sounds important for freeing memory that's no longer needed
		let previous_frame_end = Some(sync::now(device.clone()).boxed());

		Self {
			instance,
			surface,
			physical_device_index,
			device,
			graphics_queue: queue,
			swapchain,
			swapchain_images,

			data: render_data,

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
			// TODO actually have a setting for present mode somewhere instead of changing it here
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

	fn rebuild_swapchain(&mut self) {
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
			self.data.render_pass_output.clone(),
			&mut self.data.dynamic_state,
		);
		self.recreate_swapchain = false;
	}

	fn build_command_buffer(&mut self, mut frame: FrameBuilder, camera: &Camera, image_num: usize)
			-> AutoCommandBuffer<StandardCommandPoolAlloc> {
		let time = frame.get_time();

		let (vert, ind) = frame.get_sprite_renderer().get_buffers();

		// TODO don't unwrap these
		let vert_buf = self.data.vertex_buffer_pool.chunk(vert.into_iter().cloned()).unwrap();
		let ind_buf = self.data.index_buffer_pool.chunk(ind.into_iter().cloned()).unwrap();

		let transformation_matrix = camera.get_sprite_matrix();

		let push_constants = shaders::vs::ty::PushConstants {
			time,
			_dummy0: [0u8; 12],
			transform: transformation_matrix.into(),
		};

		let push_constants_output = shaders::fs_output::ty::PushConstants {
			time
		};

		let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];

		let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
			self.device.clone(),
			self.graphics_queue.family(),
		).unwrap();

		builder
			.begin_render_pass(self.data.framebuffer_main.clone(), false, clear_values.clone())
			.unwrap()
			.draw_indexed(
				self.data.pipeline_main.clone(),
				&DynamicState::none(),
				vec![Arc::new(vert_buf)],
				ind_buf,
				(),
				push_constants
			)
			.unwrap()
			.end_render_pass()
			.unwrap();

		// TODO we probably need a pipeline barrier or whatever it's called here?

		builder
			.begin_render_pass(self.framebuffers_output[image_num].clone(), false, clear_values)
			.unwrap()
			.draw(
				self.data.pipeline_output.clone(),
				&self.data.dynamic_state,
				vec![self.data.vertex_buffer_square.clone()],
				self.data.descriptor_set_output.clone(),
				push_constants
			)
			.unwrap()
			.end_render_pass()
			.unwrap();

		builder.build().unwrap()
	}

	pub fn draw_frame(&mut self, mut frame: FrameBuilder, camera: &Camera) {
		// Free resources that are no longer needed? :shrug:
		self.previous_frame_end.as_mut().unwrap().cleanup_finished();

		if self.recreate_swapchain {
			self.rebuild_swapchain();
		}

		let (image_num, suboptimal, acquire_future) =
			match swapchain::acquire_next_image(self.swapchain.clone(), None) {
				Ok(r) => r,
				Err(AcquireError::OutOfDate) => {
					self.recreate_swapchain = true;
					return;
				}
				Err(e) => panic!("Failed to acquire next swapchain image: {:?}", e)
			};

		if suboptimal {
			self.recreate_swapchain = true;
		}

		let command_buffer = self.build_command_buffer(frame, camera, image_num);

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
