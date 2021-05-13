use spin_sleep::LoopHelper;
use winit::event::{Event, WindowEvent, ElementState};
use winit::event_loop::{ControlFlow, EventLoop};

use rust_game_engine::util::timing::TickTiming;
use rust_game_engine::game::Game;
use rust_game_engine::render::renderer2::Renderer;
use futures::executor::block_on;

fn main() {
	env_logger::init();
	let events_loop = EventLoop::new();
	// let mut renderer = Renderer::init(&events_loop);
	let mut renderer = block_on(Renderer::new(&events_loop));
	let mut game = Game::new();

	// TODO why does CPU usage get maxed when using FIFO present mode and target rate is above monitor FPS?
	let mut timer = LoopHelper::builder()
		.report_interval_s(0.5)
		.build_with_target_rate(120.0);
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
			Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
				renderer.resize(size);
			},
			Event::WindowEvent { event: WindowEvent::ScaleFactorChanged {new_inner_size, ..}, ..} => {
				renderer.resize(*new_inner_size);
			},
			Event::WindowEvent { event: WindowEvent::KeyboardInput {input, .. }, .. } => {
				println!("{:?}", input);
				if let Some(key) = input.virtual_keycode {
					match input.state {
						ElementState::Pressed => {
							game.input.buffer_keydown(key);
						},
						ElementState::Released => {
							game.input.buffer_keyup(key);
						}
					}
				} else {
					// Uncomment this when the above println is gone
					// println!("weird keyboard event without a virtual keycode:\n{:?}", input);
				}
			},
			Event::RedrawEventsCleared => {},
			Event::MainEventsCleared => {
				let delta = timer.loop_start();
				tick_timer.add_delta(delta);

				time += delta.as_secs_f64();

				if let Some(fps) = timer.report_rate() {
					println!("FPS: {}", fps);
				}

				while tick_timer.try_consume_tick() {
					game.tick(tick_count);
					tick_count += 1;
				}

				game.draw_frame(&mut renderer, tick_count, tick_timer.get_partial_ticks() as f32, time as f32);
				timer.loop_sleep();
			},
			_ => ()
		}
	});
}
