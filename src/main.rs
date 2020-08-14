use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::device::{Device, DeviceExtensions, Features, QueuesIter};
use std::sync::Arc;

fn main() {
    let instance = Instance::new(None, &InstanceExtensions::none(), None)
        .expect("Failed to create Vulkan instance.");

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
}
