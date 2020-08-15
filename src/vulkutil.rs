use std::sync::Arc;

use vulkano::instance::{Instance, PhysicalDevice};

pub fn select_physical_device(instance: &Arc<Instance>) -> PhysicalDevice {
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