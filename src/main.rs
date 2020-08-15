use std::sync::Arc;

use vulkano::device::{Device, DeviceExtensions, Features, QueuesIter};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
	let instance = {
		let extensions = vulkano_win::required_extensions();
		Instance::new(None, &extensions, None)
			.expect("Failed to create Vulkan instance.")
	};

	let devices: Vec<PhysicalDevice> = PhysicalDevice::enumerate(&instance).collect();

	println!("{} devices found:", devices.len());
	for dev in &devices {
		println!("{:?} ({}, type: {:?})", dev, dev.name(), dev.ty());
	}

	let physical = *devices.get(0).expect("Failed to find any available devices");

	println!("Selected device: {}", physical.name());

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


	let events_loop = EventLoop::new();
	let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();

	events_loop.run(|event, _, control_flow| {
		match event {
			Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
				*control_flow = ControlFlow::Exit;
			},
			_ => ()
		}
	});
}
