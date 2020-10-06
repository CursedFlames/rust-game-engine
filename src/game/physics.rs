use hecs::{World, Entity};


pub trait PhysicsActor {
	fn x(&self) -> i32;
	fn y(&self) -> i32;
	fn get_move_x(&self) -> i32;
	fn get_move_y(&self) -> i32;
	fn apply_move(&mut self, x: i32, y: i32);
}

pub trait PhysicsSolid {
	fn mov(&mut self, x: f64, y: f64);
}

pub struct DebugPhysicsActor {
	pub x: i32, pub y: i32
}

impl PhysicsActor for DebugPhysicsActor {
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
}

pub struct PhysicsActorComponent(pub Box<dyn PhysicsActor + Send + Sync>);
pub struct PhysicsSolidComponent(pub Box<dyn PhysicsSolid + Send + Sync>);
// TODO do these work as type aliases instead of tuple structs?
//      couldn't get it to work when I tried
// pub type PhysicsActorComponent = Box<dyn PhysicsActor + Send + Sync>;
// pub type PhysicsSolidComponent = Box<dyn PhysicsSolid + Send + Sync>;

fn tick_solids(solids: &mut Vec<(Entity, &mut PhysicsSolidComponent)>,
		actors: &mut Vec<(Entity, &mut PhysicsActorComponent)>) {
	// TODO
}

fn tick_actors(solids: &Vec<(Entity, &mut PhysicsSolidComponent)>,
		actors: &mut Vec<(Entity, &mut PhysicsActorComponent)>) {
	for (_, ref mut actor) in actors.iter_mut() {
		// get movement
		// get expanded AABB
		// do collision checks
		// move as much as possible
		let actor = &mut actor.0;
		let dx = actor.get_move_x();
		actor.apply_move(dx, 0);
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