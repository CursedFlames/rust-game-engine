use std::thread;
use std::time::{Duration, Instant};

use spin_sleep::LoopHelper;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};

use vulkan_test::render::renderer::Renderer;
use vulkan_test::util::timing::TickTiming;

fn main() {
	let events_loop = EventLoop::new();
	let mut renderer = Renderer::init(&events_loop);
	// let mut timing = Timing::new();
	let mut timer = LoopHelper::builder()
		.report_interval_s(0.5)
		.build_with_target_rate(60.0);
	let mut tick_timer = TickTiming::new(1.0/60.0);

	let mut tick_count = 0_u32;
	let mut time = 0.0;

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
				timer.loop_sleep();

				let delta = timer.loop_start();
				tick_timer.add_delta(delta);

				time += delta.as_secs_f64();

				if let Some(fps) = timer.report_rate() {
					println!("FPS: {}", fps);
				}

				// // TODO actually do tick stuff
				while tick_timer.try_consume_tick() {
					if tick_count % 60 == 0 {
						println!("tick {}", tick_count);
					}
					tick_count += 1;
				}

				renderer.draw_frame(time as f32);
			},
			_ => ()
		}
	});
}
