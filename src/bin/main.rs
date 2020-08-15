use std::sync::Arc;

use vulkano::device::{Device, DeviceExtensions, Features, QueuesIter};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::swapchain::Surface;
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use vulkan_test::vulkutil;

fn create_window(instance: &Arc<Instance>) -> (EventLoop<()>, Arc<Surface<Window>>) {
	let events_loop = EventLoop::new();
	let surface = WindowBuilder::new()
		.with_title("Vulkan")
		.build_vk_surface(&events_loop, instance.clone())
		.unwrap();
	(events_loop, surface)
}

fn main() {
	let instance = {
		let extensions = vulkano_win::required_extensions();
		Instance::new(None, &extensions, None)
			.expect("Failed to create Vulkan instance.")
	};

	let physical = vulkutil::select_physical_device(&instance);

	let queue_family = physical.queue_families()
		.find(|&q| q.supports_graphics())
		.expect("Failed to find a graphical queue family");

	println!("Selected queue family: {:?}", queue_family);

	let (device, mut queues) = {
		Device::new(physical, &Features ::none(), &DeviceExtensions::none(),
					[(queue_family, 0.5)].iter().cloned()).expect("Failed to create device")
	};

	// We only have one queue
	let queue = queues.next().unwrap();

	let (events_loop, surface) = create_window(&instance);

	events_loop.run(|event, _, control_flow| {
		// TODO this might break things once we're actually rendering to the surface
		//      for now it just serves to prevent max CPU usage on one thread
		*control_flow = ControlFlow::Wait;
		// Commented out to prevent console spam
		// println!("event {:?}", event);
		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				*control_flow = ControlFlow::Exit;
			},
			Event::WindowEvent { event: WindowEvent::Resized(size), ..} => {
				println!("resized {:?}", size);
			}
			_ => ()
		}
	});
}
