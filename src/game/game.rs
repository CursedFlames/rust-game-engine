

pub struct Game {

}

impl Game {
	pub fn new() -> Self {
		Game {}
	}

	pub fn tick(&mut self, tick_count: u32) {
		if tick_count % 60 == 0 {
			println!("Game tick!");
		}
	}
}