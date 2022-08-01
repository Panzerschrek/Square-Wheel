use common::{bbox::*, bsp_map_compact, math_types::*};
use rapier3d::prelude as r3d;
use std::sync::Arc;

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
}

pub type ObjectHandle = r3d::RigidBodyHandle;

impl TestGamePhysics
{
	pub fn new(map: Arc<bsp_map_compact::BSPMap>) -> Self
	{
		let rigid_body_set = r3d::RigidBodySet::new();
		let mut collider_set = r3d::ColliderSet::new();

		// Static geometry.
		collider_set.insert(make_map_collider(&map));

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
		}
	}

	pub fn add_object(&mut self, position: &Vec3f, rotation: &EulerAnglesF, bbox: &BBox) -> ObjectHandle
	{
		// TODO - maybe tune physics and disable CCD?
		let body = r3d::RigidBodyBuilder::dynamic()
			.translation(r3d::Vector::new(position.x, position.y, position.z))
			.rotation(r3d::AngVector::new(rotation.x.0, rotation.y.0, rotation.z.0))
			.ccd_enabled(true)
			.build();

		let bbox_half_size = bbox.get_size() * 0.5;
		let bbox_center = bbox.get_center();

		let collider = r3d::ColliderBuilder::cuboid(bbox_half_size.x, bbox_half_size.y, bbox_half_size.z)
			.translation(r3d::Vector::new(bbox_center.x, bbox_center.y, bbox_center.z))
			.restitution(0.7)
			.build();

		let handle = self.rigid_body_set.insert(body);
		self.collider_set
			.insert_with_parent(collider, handle, &mut self.rigid_body_set);

		handle
	}

	pub fn remove_object(&mut self, handle: ObjectHandle)
	{
		self.rigid_body_set.remove(
			handle,
			&mut self.island_manager,
			&mut self.collider_set,
			&mut self.impulse_joint_set,
			&mut self.multibody_joint_set,
			true,
		);
	}

	pub fn get_object_location(&self, handle: ObjectHandle) -> (Vec3f, EulerAnglesF)
	{
		let body = &self.rigid_body_set[handle];
		let position = body.position();
		let translation = position.translation;
		let rotation = position.rotation;
		let rotation_xyzw = rotation.coords;

		(
			Vec3f::new(translation.x, translation.y, translation.z),
			EulerAnglesF::from(QuaternionF::from_sv(
				rotation_xyzw[3],
				Vec3f::new(rotation_xyzw[0], rotation_xyzw[1], rotation_xyzw[2]),
			)),
		)
	}

	pub fn update(&mut self, time_delta_s: f32)
	{
		// Perform several physics steps in case of low FPS.
		let max_dt = 1.0 / 30.0;
		let mut cur_step_time = 0.0;
		while cur_step_time < time_delta_s
		{
			let cur_dt = (time_delta_s - cur_step_time).min(max_dt);
			cur_step_time += max_dt;

			let gravity = r3d::Vector::new(0.0, 0.0, -9.81);
			let mut integration_parameters = r3d::IntegrationParameters::default();
			integration_parameters.dt = cur_dt;

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
		}
	}
}

fn make_map_collider(map: &bsp_map_compact::BSPMap) -> r3d::Collider
{
	let vertices = map
		.vertices
		.iter()
		.map(|v| r3d::Point::new(v.x, v.y, v.z))
		.collect::<Vec<_>>();

	let mut indices = Vec::new();
	for polygon in &map.polygons
	{
		for i in 0 .. polygon.num_vertices - 2
		{
			indices.push([
				polygon.first_vertex + 0,
				polygon.first_vertex + i + 1,
				polygon.first_vertex + i + 2,
			]);
		}
	}

	// TODO - ignore submodels polygons and polygons without collisions.

	r3d::ColliderBuilder::trimesh(vertices, indices).build()
}
