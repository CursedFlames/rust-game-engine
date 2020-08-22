use std::thread;
use std::time::{Duration, Instant};

use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use vulkan_test::render::renderer::Renderer;

fn main() {
	let events_loop = EventLoop::new();

	let mut renderer = Renderer::init(&events_loop);

	let start_instant = Instant::now();
	// let mut frame_count = 0;
	let mut last_print_time = 0.0;
	let mut frames_since_last_print = 0;

	events_loop.run(move |event, _, control_flow| {
		// Not used since it means framerate is kept low unless events are occurring
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
				// frame_count += 1;
				frames_since_last_print += 1;

				let elapsed = start_instant.elapsed().as_secs_f32();

				renderer.draw_frame(elapsed);

				// Janky solution to prevent max CPU usage for now (sleep for 10ms)
				thread::sleep(Duration::new(0, 10000000));
			},
			_ => ()
		}
	});
}
