use hecs::{World, Entity};
use crate::util::aabb::Aabb;
use crate::util::aabb::layer::CollisionLayers;

pub trait PhysicsElem {
	fn x(&self) -> i32;
	fn y(&self) -> i32;
	fn get_move_x(&self) -> i32;
	fn get_move_y(&self) -> i32;
	fn apply_move(&mut self, x: i32, y: i32);

	fn get_base_aabb(&self) -> Aabb;
	fn get_aabb(&self) -> Aabb;

	fn get_collision_layers(&self) -> CollisionLayers;
}

pub trait PhysicsActor : PhysicsElem {
}

pub trait PhysicsSolid : PhysicsElem {
}

pub struct DebugPhysicsActor {
	pub x: i32, pub y: i32
}

impl PhysicsElem for DebugPhysicsActor {
	fn x(&self) -> i32 {self.x}
	fn y(&self) -> i32 {self.y}
	fn get_move_x(&self) -> i32 {
		1
	}
	fn get_move_y(&self) -> i32 {
		1
	}
	fn apply_move(&mut self, x: i32, y: i32) {
		self.x += x;
		self.y += y;
	}

	fn get_base_aabb(&self) -> Aabb {
		Aabb::from_width_height(8, 8)
	}
	fn get_aabb(&self) -> Aabb {
		Aabb::from_pos_width_height(self.x, self.y, 8, 8)
	}

	fn get_collision_layers(&self) -> CollisionLayers {
		u32::MAX
	}
}
impl PhysicsActor for DebugPhysicsActor {}

pub type PhysicsActorComponent = Box<dyn PhysicsActor + Send + Sync>;
pub type PhysicsSolidComponent = Box<dyn PhysicsSolid + Send + Sync>;

fn collisions(aabb: Aabb, layers: CollisionLayers, solids: &Vec<(Entity, &mut PhysicsSolidComponent)>)
		-> Vec<Aabb> {
	// TODO maybe we want some sort of should_collide(solid, actor) for finer control?
	let mut collisions = Vec::new();
	if layers == 0 {
		return collisions
	}
	for (_, solid) in solids.iter() {
		if (solid.get_collision_layers() & layers) != 0 {
			let aabb2 = solid.get_aabb();
			if aabb.intersects(aabb2) {
				collisions.push(aabb2);
			}
		}
	}
	// TODO might want to also return CollisionLayers, or maybe the entire Solid
	collisions
}

fn tick_solids(solids: &mut Vec<(Entity, &mut PhysicsSolidComponent)>,
		actors: &mut Vec<(Entity, &mut PhysicsActorComponent)>) {
	// TODO
}

fn tick_actors(solids: &Vec<(Entity, &mut PhysicsSolidComponent)>,
		actors: &mut Vec<(Entity, &mut PhysicsActorComponent)>) {
	for (_, ref mut actor) in actors.iter_mut() {
		let collision_layers = actor.get_collision_layers();
		// TODO optimize out collision checks if layers is 0?
		// get movement
		// get expanded AABB
		// do collision checks
		// move as much as possible
		let dx = actor.get_move_x();
		let start_pos = actor.get_aabb();
		let end_pos = start_pos.offset(dx, 0);
		let aabb = Aabb::union(start_pos, end_pos);
		let intersects = collisions(aabb, collision_layers, solids);
		if !intersects.is_empty() {
			// TODO some functionality to squish around corners?
			let mut offset: i32 = 0;
			'outer: while offset.abs() < dx.abs() {
				let aabb = start_pos.offset(offset+1, 0);
				for aabb2 in intersects.iter() {
					if aabb.intersects(*aabb2) {
						break 'outer;
					}
				}
				offset += 1;
			}
			actor.apply_move(offset, 0);
		} else {
			actor.apply_move(dx, 0);
		}
		let dy = actor.get_move_y();
		actor.apply_move(0, dy);
	}
}

pub fn tick_physics(level: &mut World) {
	// TODO apply physics for all solids, then all actors
	//      Solids should move x first, then y
	//      apply all solid horizontals, then all solid verticals
	// TODO maybe have solids mark on actors what movement will be applied to them, before it's applied?
	//      to prevent non-determinism when multiple solids push an actor
	//      separate the horizontal and vertical steps for this, though, in case an entity gets pushed
	//      into the path of a second solid
	//      but that still causes issues with non-deterministic behaviour, god damn it
	// Entities with both Solid and Actor are undefined behavior
	// filter out entities with both using .without() to prevent weird panics
	let mut solids_query = level
		.query::<&mut PhysicsSolidComponent>()
		.without::<&PhysicsActorComponent>();
	let mut solids = solids_query.iter().collect::<Vec<_>>();
	let mut actors_query = level
		.query::<&mut PhysicsActorComponent>()
		.without::<&PhysicsSolidComponent>();
	let mut actors = actors_query.iter().collect::<Vec<_>>();
	tick_solids(&mut solids, &mut actors);
	tick_actors(&solids, &mut actors);

}