use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::{DescriptorSetDesc, PersistentDescriptorSet};
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPass, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage, StorageImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use vulkano::swapchain::{self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use vulkan_test::vulkutil;
use vulkano::image::Dimensions::Dim2d;

fn create_window(instance: &Arc<Instance>) -> (EventLoop<()>, Arc<Surface<Window>>) {
	let events_loop = EventLoop::new();
	let surface = WindowBuilder::new()
		.with_title("Vulkan")
		.build_vk_surface(&events_loop, instance.clone())
		.unwrap();
	(events_loop, surface)
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
		Device::new(physical, &Features ::none(), &device_ext,
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

fn main() {
	let instance = {
		let extensions = vulkano_win::required_extensions();
		Instance::new(None, &extensions, None)
			.expect("Failed to create Vulkan instance.")
	};

	let physical = vulkutil::select_physical_device(&instance);

	let (events_loop, surface) = create_window(&instance);

	let (device, queue) = get_device_and_queue(physical);

	let (mut swapchain, images) =
		create_swapchain(physical, &device, &surface, &queue);

	let mut intermediate_image = AttachmentImage::with_usage(
		device.clone(),
		[320, 180],
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

	// TODO move this Vertex struct to somewhere more sensible
	//      ideally move all this buffer stuff somewhere more sensible, really.
	#[derive(Default, Debug, Clone)]
	struct Vertex {
		position: [f32; 2],
	}
	vulkano::impl_vertex!(Vertex, position);

	let vertex_buffer_triangle = {
		CpuAccessibleBuffer::from_iter(
			device.clone(),
			// TODO pick actual BufferUsage
			BufferUsage::all(),
			false,
			[
				Vertex {position: [-0.5, -0.25]},
				Vertex {position: [0.0, 0.5]},
				Vertex {position: [0.25, -0.1]},
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
				Vertex {position: [-1.0, -1.0]},
				Vertex {position: [-1.0, 1.0]},
				Vertex {position: [1.0, -1.0]},
				Vertex {position: [1.0, 1.0]},
				Vertex {position: [-1.0, 1.0]},
				Vertex {position: [1.0, -1.0]},
			].iter().cloned()
		).unwrap()
	};

	// TODO figure out where to put shaders
	//      apparently they don't rebuild when changed unless other things in the file are also changed?
	//      irritating if true.
	mod vs {
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

	mod fs_triangle {
		vulkano_shaders::shader! {
			ty: "fragment",
			src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

void main() {
	f_color = vec4(0.1, 0.25, 1.0, 1.0);
}"
		}
	}

	mod fs_output {
		vulkano_shaders::shader! {
			ty: "fragment",
			src: "\
#version 450
layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 f_color;

layout(binding = 0) uniform unf_data {
	float time;
};
layout(binding = 1) uniform sampler2D texSampler;
void main() {
	// f_color = vec4(sin(time/4.0), 0.25, 1.0, 1.0);
	f_color = texture(texSampler, fragTexCoord);
}"
		}
	}

	let fragment_uniform_buffer = CpuBufferPool::<fs_output::ty::unf_data>::new(device.clone(), BufferUsage::all());

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
			.vertex_input_single_buffer::<Vertex>()
			.vertex_shader(vs.main_entry_point(), ())
			.triangle_list()
			.viewports(vec![Viewport {
				origin: [0.0, 0.0],
				dimensions: [320.0, 180.0],
				depth_range: 0.0..1.0,
			}])
			.fragment_shader(fs_triangle.main_entry_point(), ())
			.render_pass(Subpass::from(render_pass_main.clone(), 0).unwrap())
			.build(device.clone())
			.unwrap()
	);

	let pipeline_output = Arc::new(
		GraphicsPipeline::start()
			.vertex_input_single_buffer::<Vertex>()
			.vertex_shader(vs.main_entry_point(), ())
			.triangle_list()
			.viewports_dynamic_scissors_irrelevant(1)
			.fragment_shader(fs_output.main_entry_point(), ())
			.render_pass(Subpass::from(render_pass_output.clone(), 0).unwrap())
			.build(device.clone())
			.unwrap()
	);

	let dynamic_state_none = DynamicState::none();

	// Need this for dynamically updating the viewport when resizing the window.
	let mut dynamic_state = dynamic_state_none.clone();

	let framebuffer_main = Arc::new(
		Framebuffer::start(render_pass_main)
			.add(intermediate_image.clone())
			.unwrap()
			.build()
			.unwrap()
	);

	let mut framebuffers_output = window_size_dependent_setup(&images, render_pass_output.clone(), &mut dynamic_state);

	let mut recreate_swapchain = false;
	// I'm not clear on what exactly this does, but it sounds important for freeing memory that's no longer needed
	let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

	let start_instant = Instant::now();
	let mut frame_count = 0;
	let mut last_print_time = 0.0;
	let mut frames_since_last_print = 0;

	events_loop.run(move |event, _, control_flow| {
		// TODO this might break things once we're actually rendering to the surface
		//      for now it just serves to prevent max CPU usage on one thread
		// *control_flow = ControlFlow::Wait;
		// Commented out to prevent console spam
		// println!("event {:?}", event);
		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				*control_flow = ControlFlow::Exit;
			},
			Event::WindowEvent { event: WindowEvent::Resized(size), ..} => {
				println!("resized {:?}", size);
				recreate_swapchain = true;
			},
			Event::RedrawEventsCleared => {},
			Event::MainEventsCleared => {
				let time_elapsed = start_instant.elapsed().as_secs_f64();
				if time_elapsed > last_print_time + 1.0 {
					println!("{} FPS", frames_since_last_print);
					last_print_time = time_elapsed;
					frames_since_last_print = 0;
				}
				frame_count += 1;
				frames_since_last_print += 1;

				// Free resources that are no longer needed? :shrug:
				previous_frame_end.as_mut().unwrap().cleanup_finished();

				if recreate_swapchain {
					let dimensions: [u32; 2] = surface.window().inner_size().into();
					let (new_swapchain, new_images) =
						match swapchain.recreate_with_dimensions(dimensions) {
							Ok(r) => r,
							// This tends to happen while the user is resizing the window, apparently
							Err(SwapchainCreationError::UnsupportedDimensions) => return,
							Err(e) => panic!("Failed to recreate swapchain: {:?}", e)
						};

					swapchain = new_swapchain;
					framebuffers_output = window_size_dependent_setup(
						&new_images,
						render_pass_output.clone(),
						&mut dynamic_state,
					);
					recreate_swapchain = false;
				}

				let (image_num, suboptimal, acquire_future) =
					match swapchain::acquire_next_image(swapchain.clone(), None) {
						Ok(r) => r,
						Err(AcquireError::OutOfDate) => {
							recreate_swapchain = true;
							return;
						}
						Err(e) => panic!("Failed to acquire next image: {:?}", e)
					};

				if suboptimal {
					recreate_swapchain = true;
				}

				let fragment_uniform_subbuf = {
					let elapsed = start_instant.elapsed().as_secs_f32();

					let fragment_data = fs_output::ty::unf_data {
						time: elapsed
					};
					fragment_uniform_buffer.next(fragment_data).unwrap()
				};

				let layout = pipeline_output.layout().descriptor_set_layout(0).expect("Failed to get set layout");

				// TODO we probably want to do these descriptors elsewhere when possible?
				let uniform_set = Arc::new(
					PersistentDescriptorSet::start(layout.clone())
						.add_buffer(fragment_uniform_subbuf)
						.expect("Failed to add fragment uniform buffer")
						.add_sampled_image(intermediate_image.clone(), sampler_simple_nearest.clone())
						.expect("Failed to add sampled image")
						.build()
						.expect("Failed to build uniform object"),
				);

				let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];

				let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
					device.clone(),
					queue.family(),
				).unwrap();

				builder
					.begin_render_pass(framebuffer_main.clone(), false, clear_values.clone())
					.unwrap()
					.draw(
						pipeline_main.clone(),
						&dynamic_state_none,
						vertex_buffer_triangle.clone(),
						(),
						()
					)
					.unwrap()
					.end_render_pass()
					.unwrap();

				builder
					.begin_render_pass(framebuffers_output[image_num].clone(), false, clear_values)
					.unwrap()
					.draw(
						pipeline_output.clone(),
						&dynamic_state,
						vertex_buffer_square.clone(),
						uniform_set,
						(),
					)
					.unwrap()
					.end_render_pass()
					.unwrap();

				let command_buffer = builder.build().unwrap();

				let future = previous_frame_end
					.take()
					.unwrap()
					.join(acquire_future)
					.then_execute(queue.clone(), command_buffer)
					.unwrap()
					.then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
					.then_signal_fence_and_flush();

				match future {
					Ok(future) => {
						previous_frame_end = Some(future.boxed());
					},
					Err(FlushError::OutOfDate) => {
						recreate_swapchain = true;
						previous_frame_end = Some(sync::now(device.clone()).boxed());
					},
					Err(e) => {
						println!("Failed to flush future: {:?}", e);
						previous_frame_end = Some(sync::now(device.clone()).boxed());
					}
				}

				// Janky solution to prevent max CPU usage for now (sleep for 10ms)
				thread::sleep(Duration::new(0, 10000000));
			},
			_ => ()
		}
	});
}
