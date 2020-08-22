use std::thread;
use std::time::{Duration, Instant};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use vulkan_test::render::renderer::Renderer;
use vulkan_test::util::timing::Timing;

fn main() {
	let events_loop = EventLoop::new();
	let mut renderer = Renderer::init(&events_loop);
	let mut timing = Timing::new();

	// let mut tick_count = 0;

	events_loop.run(move |event, _, control_flow| {
		// Not used since it means framerate is kept low unless events are occurring
		// We might want to use it when window is minimized, etc.?
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
				timing.wait_for_next_frame();
				/*let delta = */timing.on_update();
				let time = timing.get_total_time();

				// TODO actually do tick stuff
				while timing.try_consume_tick() {
					// println!("tick {}", tick_count);
					// tick_count += 1;
				}

				renderer.draw_frame(time as f32);
			},
			_ => ()
		}
	});
}
