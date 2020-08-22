use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions, Features, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPass, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage, StorageImage, SwapchainImage};
use vulkano::image::Dimensions::Dim2d;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode};
use vulkano::swapchain::{self, AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain, SwapchainCreationError};
use vulkano::sync::{self, FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use vulkan_test::render::renderer::{self, Renderer};
use vulkan_test::render::vert::Vertex2d;
use vulkan_test::vulkutil;

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
	let events_loop = EventLoop::new();

	let mut renderer = Renderer::init(&events_loop);

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
				renderer.recreate_swapchain = true;
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
				renderer.previous_frame_end.as_mut().unwrap().cleanup_finished();

				if renderer.recreate_swapchain {
					let dimensions: [u32; 2] = renderer.surface.window().inner_size().into();
					let (new_swapchain, new_images) =
						match renderer.swapchain.recreate_with_dimensions(dimensions) {
							Ok(r) => r,
							// This tends to happen while the user is resizing the window, apparently
							Err(SwapchainCreationError::UnsupportedDimensions) => return,
							Err(e) => panic!("Failed to recreate swapchain: {:?}", e)
						};

					renderer.swapchain = new_swapchain;
					renderer.framebuffers_output = window_size_dependent_setup(
						&new_images,
						renderer.render_pass_output.clone(),
						&mut renderer.dynamic_state,
					);
					renderer.recreate_swapchain = false;
				}

				let (image_num, suboptimal, acquire_future) =
					match swapchain::acquire_next_image(renderer.swapchain.clone(), None) {
						Ok(r) => r,
						Err(AcquireError::OutOfDate) => {
							renderer.recreate_swapchain = true;
							return;
						}
						Err(e) => panic!("Failed to acquire next image: {:?}", e)
					};

				if suboptimal {
					renderer.recreate_swapchain = true;
				}

				let elapsed = start_instant.elapsed().as_secs_f32();
				let push_constants = renderer::fs_output::ty::PushConstants {
					time: elapsed
				};

				let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into()];

				let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
					renderer.device.clone(),
					renderer.graphics_queue.family(),
				).unwrap();

				builder
					.begin_render_pass(renderer.framebuffer_main.clone(), false, clear_values.clone())
					.unwrap()
					.draw(
						renderer.pipeline_main.clone(),
						&DynamicState::none(),
						vec![renderer.vertex_buffer_triangle.clone()],
						(),
						push_constants
					)
					.unwrap()
					.end_render_pass()
					.unwrap();

				builder
					.begin_render_pass(renderer.framebuffers_output[image_num].clone(), false, clear_values)
					.unwrap()
					.draw(
						renderer.pipeline_output.clone(),
						&renderer.dynamic_state,
						vec![renderer.vertex_buffer_square.clone()],
						renderer.descriptor_set_output.clone(),
						push_constants
					)
					.unwrap()
					.end_render_pass()
					.unwrap();

				let command_buffer = builder.build().unwrap();

				let future = renderer.previous_frame_end
					.take()
					.unwrap()
					.join(acquire_future)
					.then_execute(renderer.graphics_queue.clone(), command_buffer)
					.unwrap()
					.then_swapchain_present(renderer.graphics_queue.clone(), renderer.swapchain.clone(), image_num)
					.then_signal_fence_and_flush();

				match future {
					Ok(future) => {
						renderer.previous_frame_end = Some(future.boxed());
					},
					Err(FlushError::OutOfDate) => {
						renderer.recreate_swapchain = true;
						renderer.previous_frame_end = Some(sync::now(renderer.device.clone()).boxed());
					},
					Err(e) => {
						println!("Failed to flush future: {:?}", e);
						renderer.previous_frame_end = Some(sync::now(renderer.device.clone()).boxed());
					}
				}

				// Janky solution to prevent max CPU usage for now (sleep for 10ms)
				thread::sleep(Duration::new(0, 10000000));
			},
			_ => ()
		}
	});
}
