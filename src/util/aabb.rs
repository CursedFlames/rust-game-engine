use std::cmp::{min, max};

pub struct Aabb {
	pub x1: i32,
	pub y1: i32,
	pub x2: i32,
	pub y2: i32
}

impl Aabb {
	fn from_width_height(width: i32, height: i32) -> Self {
		// Empty AABBs are undefined behavior
		assert!(width > 0 && height > 0);
		Aabb {
			x1: 0,
			y1: 0,
			x2: width-1,
			y2: height-1,
		}
	}

	fn from_pos_width_height(x: i32, y: i32, width: i32, height: i32) -> Self {
		// Empty AABBs are undefined behavior
		assert!(width > 0 && height > 0);
		Aabb {
			x1: x,
			y1: y,
			x2: x + width-1,
			y2: y + height-1
		}
	}

	fn offset4(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
		Aabb {
			x1: self.x1 + x1,
			y1: self.y1 + y1,
			x2: self.x2 + x2,
			y2: self.y2 + y2
		}
	}

	fn offset(&self, x: i32, y: i32) -> Self {
		self.offset4(x, y, x, y)
	}

	fn grow(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
		self.offset4(-x1, -y1, x2, y2)
	}

	fn containing_both(a: Self, b: Self) -> Self {
		Aabb {
			x1: min(a.x1, b.x1),
			y1: min(a.y1, b.y1),
			x2: max(a.x2, b.x2),
			y2: max(a.y2, b.y2),
		}
	}

	fn intersects(&self, other: Self) -> bool {
		self.x2 >= other.x1
			&& self.y2 >= other.y1
			&& self.x1 <= other.x2
			&& self.y1 <= other.y2
	}
}