use rapier3d::prelude as r3d;

pub struct TestGamePhysics
{
	rigid_body_set: r3d::RigidBodySet,
	collider_set: r3d::ColliderSet,

	physics_pipeline: r3d::PhysicsPipeline,
	island_manager: r3d::IslandManager,
	broad_phase: r3d::BroadPhase,
	narrow_phase: r3d::NarrowPhase,
	impulse_joint_set: r3d::ImpulseJointSet,
	multibody_joint_set: r3d::MultibodyJointSet,
	ccd_solver: r3d::CCDSolver,
	physics_hooks: (),
	event_handler: (),

	ball_body_handle: r3d::RigidBodyHandle,
}

impl TestGamePhysics
{
	pub fn new() -> Self
	{
		let mut rigid_body_set = r3d::RigidBodySet::new();
		let mut collider_set = r3d::ColliderSet::new();

		// Static geometry.
		collider_set.insert(r3d::ColliderBuilder::cuboid(100.0, 100.0, 10.0).build());

		// TODO - enable continuous collision detection to prevent tunnelling.

		let ball_rigid_body = r3d::RigidBodyBuilder::dynamic()
			.translation(r3d::Vector::new(0.0, 0.0, 200.0))
			.build();
		let ball_collider = r3d::ColliderBuilder::ball(0.5).restitution(0.7).build();
		let ball_body_handle = rigid_body_set.insert(ball_rigid_body);
		collider_set.insert_with_parent(ball_collider, ball_body_handle, &mut rigid_body_set);

		Self {
			rigid_body_set,
			collider_set,
			physics_pipeline: r3d::PhysicsPipeline::new(),
			island_manager: r3d::IslandManager::new(),
			broad_phase: r3d::BroadPhase::new(),
			narrow_phase: r3d::NarrowPhase::new(),
			impulse_joint_set: r3d::ImpulseJointSet::new(),
			multibody_joint_set: r3d::MultibodyJointSet::new(),
			ccd_solver: r3d::CCDSolver::new(),
			physics_hooks: (),
			event_handler: (),
			ball_body_handle,
		}
	}

	pub fn update(&mut self, time_delta_s: f32)
	{
		let gravity = r3d::Vector::new(0.0, 0.0, -9.81);
		let mut integration_parameters = r3d::IntegrationParameters::default();
		integration_parameters.dt = time_delta_s;

		self.physics_pipeline.step(
			&gravity,
			&integration_parameters,
			&mut self.island_manager,
			&mut self.broad_phase,
			&mut self.narrow_phase,
			&mut self.rigid_body_set,
			&mut self.collider_set,
			&mut self.impulse_joint_set,
			&mut self.multibody_joint_set,
			&mut self.ccd_solver,
			&self.physics_hooks,
			&self.event_handler,
		);

		let ball_body = &self.rigid_body_set[self.ball_body_handle];
		println!("Ball altitude: {}", ball_body.translation().z);
	}
}
