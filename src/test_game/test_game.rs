use super::{
	commands_processor, commands_queue, components::*, console, frame_info::*, game_interface::*, light::*,
	resources_manager::*, test_game_physics,
};
use square_wheel_lib::common::{
	bbox::*, bsp_map_compact, camera_rotation_controller::*, color::*, map_file_common, material, math_types::*,
	matrix::*, system_window,
};
use std::sync::Arc;

pub struct Game
{
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	resources_manager: ResourcesManagerSharedPtr,
	commands_queue: commands_queue::CommandsQueuePtr<Game>,
	commands_queue_dyn: commands_queue::CommandsQueueDynPtr,
	map: Arc<bsp_map_compact::BSPMap>,
	physics: test_game_physics::TestGamePhysics,
	game_time: f32,
	ecs: hecs::World,
	ecs_command_buffer: hecs::CommandBuffer,
	player_entity: hecs::Entity,
}

impl Game
{
	pub fn new(
		commands_processor: commands_processor::CommandsProcessorPtr,
		console: console::ConsoleSharedPtr,
		resources_manager: ResourcesManagerSharedPtr,
		map: Arc<bsp_map_compact::BSPMap>,
	) -> Self
	{
		let commands_queue = commands_queue::CommandsQueue::new(vec![
			("get_pos", Game::command_get_pos),
			("set_pos", Game::command_set_pos),
			("get_angles", Game::command_get_angles),
			("set_angles", Game::command_set_angles),
			("add_test_light", Game::command_add_test_light),
			("reset_test_lights", Game::command_reset_test_lights),
			("add_test_model", Game::command_add_test_model),
			("reset_test_models", Game::command_reset_test_models),
			("add_test_decal", Game::command_add_test_decal),
			("reset_test_decals", Game::command_reset_test_decals),
			("set_view_model", Game::command_set_view_model),
			("reset_view_model", Game::command_reset_view_model),
			("noclip", Game::command_noclip),
		]);

		let commands_queue_dyn = commands_queue.clone() as commands_queue::CommandsQueueDynPtr;
		commands_processor
			.lock()
			.unwrap()
			.register_command_queue(commands_queue_dyn.clone());

		let mut result = Self {
			commands_processor,
			console,
			resources_manager,
			commands_queue,
			commands_queue_dyn,
			map: map.clone(),
			physics: test_game_physics::TestGamePhysics::new(map),
			game_time: 0.0,
			ecs: hecs::World::new(),
			ecs_command_buffer: hecs::CommandBuffer::new(),
			player_entity: hecs::Entity::DANGLING,
		};

		result.spawn_entities();

		result
	}

	fn spawn_entities(&mut self)
	{
		self.spawn_regular_entities();
		self.spawn_player();
	}

	fn spawn_regular_entities(&mut self)
	{
		// Skip world entity.
		for map_entity in &self.map.entities[1 ..]
		{
			match get_entity_classname(map_entity, &self.map)
			{
				Some("func_detail") =>
				{
					// Spawn non-moving static entity.
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let entity = self.ecs.spawn((SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity {
								position: bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index])
									.get_center(),
								rotation: QuaternionF::zero(),
							},
						},));
						self.ecs
							.insert_one(
								entity,
								self.physics
									.add_submodel_object(entity, index, &Vec3f::zero(), &QuaternionF::zero()),
							)
							.ok();

						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);
					}
				},
				Some("trigger_multiple") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let entity = self.ecs.spawn((TouchTriggerComponent { bbox },));
						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);
					}
				},
				Some("path_corner") =>
				{
					if let Some(origin_str) = get_entity_key_value(map_entity, &self.map, "origin")
					{
						if let Ok(origin) = map_file_common::parse_vec3(origin_str)
						{
							let entity = self.ecs.spawn((
								LocationComponent {
									position: origin,
									rotation: QuaternionF::zero(),
								},
								WaitComponent {
									wait: get_entity_f32(map_entity, &self.map, "wait").unwrap_or(0.0),
								},
							));

							Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);
						}
					}
				},
				Some("func_plat") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let height = if let Some(h) = get_entity_f32(map_entity, &self.map, "height")
						{
							h
						}
						else
						{
							bbox.max.z - bbox.min.z - 8.0
						};

						let position_upper = bbox.get_center();
						let position_lower = position_upper - Vec3f::new(0.0, 0.0, height);

						let position = position_lower;
						let rotation = QuaternionF::zero();

						// TODO - start at top if plate requires activation by another entity.

						let entity = self.ecs.spawn(());
						self.ecs
							.insert(
								entity,
								(
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity { position, rotation },
									},
									LocationComponent { position, rotation },
									EntityActivationComponent { activated: false },
									PlateComponent {
										speed: get_entity_f32(map_entity, &self.map, "speed").unwrap_or(150.0),
										position_lower,
										position_upper,
										state: PlateState::TargetDown,
									},
									// Update physics object using location component.
									LocationKinematicPhysicsObjectComponent {
										phys_handle: self.physics.add_submodel_object(
											entity,
											index,
											&Vec3f::new(0.0, 0.0, -height),
											&rotation,
										),
									},
									// Update draw model location, using location component.
									SubmodelEntityWithIndexLocationLinkComponent {},
								),
							)
							.ok();

						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);

						// Add activation trigger.
						// TODO - use PLAT_LOW_TRIGGER.
						let bbox_half_size = bbox.get_size() * 0.5;
						let bbox_reduce_min = Vec3f::new(
							(bbox_half_size.x - 1.0).min(25.0),
							(bbox_half_size.y - 1.0).min(25.0),
							0.0,
						);
						let bbox_reduce_max = Vec3f::new(
							(bbox_half_size.x - 1.0).min(25.0),
							(bbox_half_size.y - 1.0).min(25.0),
							-8.0,
						);

						let trigger_bbox = BBox::from_min_max(bbox.min + bbox_reduce_min, bbox.max - bbox_reduce_max);

						self.ecs.spawn((
							TouchTriggerComponent { bbox: trigger_bbox },
							TriggerSingleTargetComponent { target: entity },
						));
					}
				},
				Some("func_door") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let direction = get_entity_move_direction(map_entity, &self.map);

						let lip = get_entity_f32(map_entity, &self.map, "lip").unwrap_or(8.0);

						// TODO - use DOOR_START_OPEN.
						let position_closed = bbox.get_center();
						let position_opened =
							position_closed + direction * (direction.dot(bbox.get_size()).abs() - lip);

						let position = position_closed;
						let rotation = QuaternionF::zero();

						let entity = self.ecs.spawn(());
						self.ecs
							.insert(
								entity,
								(
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity { position, rotation },
									},
									LocationComponent { position, rotation },
									EntityActivationComponent { activated: false },
									DoorComponent {
										speed: get_entity_f32(map_entity, &self.map, "speed").unwrap_or(100.0),
										wait: get_entity_f32(map_entity, &self.map, "wait").unwrap_or(3.0),
										position_opened,
										position_closed,
										state: DoorState::TargetClosed,
										slave_doors: Vec::new(), // set later
									},
									// Update physics object using location component.
									LocationKinematicPhysicsObjectComponent {
										phys_handle: self.physics.add_submodel_object(
											entity,
											index,
											&(bbox.get_center() - position_closed),
											&rotation,
										),
									},
									// Update draw model location, using location component.
									SubmodelEntityWithIndexLocationLinkComponent {},
								),
							)
							.ok();

						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);

						// Add trigger later - after linking touched doors.
					}
				},
				Some("func_button") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let direction = get_entity_move_direction(map_entity, &self.map);

						let lip = get_entity_f32(map_entity, &self.map, "lip").unwrap_or(4.0);

						let position_released = bbox.get_center();
						let position_pressed =
							position_released + direction * (direction.dot(bbox.get_size()).abs() - lip);

						let position = position_released;
						let rotation = QuaternionF::zero();

						let entity = self.ecs.spawn(());
						self.ecs
							.insert(
								entity,
								(
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity { position, rotation },
									},
									LocationComponent { position, rotation },
									EntityActivationComponent { activated: false },
									ButtonComponent {
										speed: get_entity_f32(map_entity, &self.map, "speed").unwrap_or(40.0),
										wait: get_entity_f32(map_entity, &self.map, "wait").unwrap_or(1.0),
										position_released,
										position_pressed,
										state: ButtonState::TargetReleased,
									},
									// Update physics object using location component.
									LocationKinematicPhysicsObjectComponent {
										phys_handle: self.physics.add_submodel_object(
											entity,
											index,
											&(bbox.get_center() - position_released),
											&rotation,
										),
									},
									// Update draw model location, using location component.
									SubmodelEntityWithIndexLocationLinkComponent {},
								),
							)
							.ok();

						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);

						let bbox_increase = Vec3f::new(1.0, 1.0, 1.0);
						let trigger_bbox = BBox::from_min_max(bbox.min - bbox_increase, bbox.max + bbox_increase);

						self.ecs.spawn((
							TouchTriggerComponent { bbox: trigger_bbox },
							TriggerSingleTargetComponent { target: entity },
						));
					}
				},
				Some("func_train") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let position = bbox.get_center();
						let rotation = QuaternionF::zero();

						let entity = self.ecs.spawn(());
						self.ecs
							.insert(
								entity,
								(
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity { position, rotation },
									},
									LocationComponent { position, rotation },
									EntityActivationComponent { activated: false },
									TrainComponent {
										speed: get_entity_f32(map_entity, &self.map, "speed").unwrap_or(100.0),
										state: TrainState::SearchForInitialPosition,
										target: entity,
										// Shift target positions because in Quake position is regulated for minimum point of bbox.
										target_shift: bbox.get_size() * 0.5,
									},
									// Update physics object using location component.
									LocationKinematicPhysicsObjectComponent {
										phys_handle: self.physics.add_submodel_object(
											entity,
											index,
											&Vec3f::zero(),
											&rotation,
										),
									},
									// Update draw model location, using location component.
									SubmodelEntityWithIndexLocationLinkComponent {},
								),
							)
							.ok();

						Self::add_entity_common_components(&mut self.ecs, &self.map, map_entity, entity);
					}
				},
				_ =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						// Spawn test submodel.
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);
						let position = bbox.get_center();
						let rotation = QuaternionF::zero();

						let entity = self.ecs.spawn(());
						self.ecs
							.insert(
								entity,
								(
									TestSubmodelComponent { index },
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity { position, rotation },
									},
									LocationComponent { position, rotation },
									// Update physics object using location component.
									LocationKinematicPhysicsObjectComponent {
										phys_handle: self
											.physics
											.add_submodel_object(entity, index, &position, &rotation),
									},
									// Update draw model location, using location component.
									SubmodelEntityWithIndexLocationLinkComponent {},
								),
							)
							.ok();
					}
				},
			}
		}

		self.prepare_linked_doors();
	}

	fn spawn_player(&mut self)
	{
		self.player_entity = self.ecs.spawn((
			PlayerComponent {
				view_model_entity: hecs::Entity::DANGLING,
			},
			LocationComponent {
				position: Vec3f::zero(),
				rotation: QuaternionF::zero(),
			},
			PlayerControllerLocationComponent {},
			PlayerControllerComponent {
				rotation_controller: CameraRotationController::new(),
				position_source: PlayerPositionSource::Noclip(Vec3f::zero()),
			},
		));
	}

	fn add_entity_common_components(
		ecs: &mut hecs::World,
		map: &bsp_map_compact::BSPMap,
		map_entity: &bsp_map_compact::Entity,
		entity: hecs::Entity,
	)
	{
		if let Some(target) = get_entity_key_value(map_entity, map, "target")
		{
			ecs.insert_one(
				entity,
				TargetNameComponent {
					name: target.to_string(),
				},
			)
			.ok();
		}

		if let Some(targetname) = get_entity_key_value(map_entity, map, "targetname")
		{
			ecs.insert_one(
				entity,
				NamedTargetComponent {
					name: targetname.to_string(),
				},
			)
			.ok();
		}
	}

	fn prepare_linked_doors(&mut self)
	{
		// Find touching doors groups.
		let mut master_doors = std::collections::HashMap::<hecs::Entity, (BBox, Vec<hecs::Entity>)>::new();
		let mut slave_doors_set = std::collections::HashSet::<hecs::Entity>::new();

		for (id0, (_door_component0, submodel_entity_with_index_component0)) in
			self.ecs.query::<(&DoorComponent, &SubmodelEntityWithIndex)>().iter()
		{
			if slave_doors_set.contains(&id0)
			{
				continue;
			}
			let mut bbox = bsp_map_compact::get_submodel_bbox(
				&self.map,
				&self.map.submodels[submodel_entity_with_index_component0.index],
			);

			let mut slave_doors = Vec::new();

			for (id1, (_door_component1, submodel_entity_with_index_component1)) in
				self.ecs.query::<(&DoorComponent, &SubmodelEntityWithIndex)>().iter()
			{
				if id0 == id1
				{
					continue;
				}
				if slave_doors_set.contains(&id0)
				{
					continue;
				}

				let bbox1 = bsp_map_compact::get_submodel_bbox(
					&self.map,
					&self.map.submodels[submodel_entity_with_index_component1.index],
				);
				if bbox.touches_or_intersects(&bbox1)
				{
					bbox.extend(&bbox1);
					slave_doors.push(id1);
					slave_doors_set.insert(id1);
				}
			}

			master_doors.insert(id0, (bbox, slave_doors));
		}

		// Spawn triggers for master doors only.
		// Slave doors will be automatically activated together with master doors.
		for (id, (bbox, slave_doors)) in master_doors.drain()
		{
			let mut q = self
				.ecs
				.query_one::<(&mut DoorComponent, Option<&NamedTargetComponent>)>(id)
				.unwrap();
			let (door_component, named_target_component) = q.get().unwrap();

			door_component.slave_doors = slave_doors;

			if named_target_component.is_some()
			{
				// This door is activated by some other trigger.
				continue;
			}
			drop(q);

			let bbox_increase = Vec3f::new(60.0, 60.0, 8.0);
			let trigger_bbox = BBox::from_min_max(bbox.min - bbox_increase, bbox.max + bbox_increase);

			self.ecs.spawn((
				TouchTriggerComponent { bbox: trigger_bbox },
				TriggerSingleTargetComponent { target: id },
			));
		}
	}

	fn update_player_entity(
		&mut self,
		keyboard_state: &system_window::KeyboardState,
		events: &[sdl2::event::Event],
		time_delta_s: f32,
	)
	{
		if let Ok(mut q) = self
			.ecs
			.query_one::<(&mut PlayerControllerComponent,)>(self.player_entity)
		{
			let (player_controller,) = q.get().unwrap();
			player_controller
				.rotation_controller
				.update(keyboard_state, events, time_delta_s);

			let azimuth = player_controller.rotation_controller.get_angles().0;
			let forward_vector = Vec3f::new(-(azimuth.sin()), azimuth.cos(), 0.0);
			let left_vector = Vec3f::new(azimuth.cos(), azimuth.sin(), 0.0);
			let mut move_vector = Vec3f::new(0.0, 0.0, 0.0);

			use sdl2::keyboard::Scancode;
			if keyboard_state.contains(&Scancode::W)
			{
				move_vector += forward_vector;
			}
			if keyboard_state.contains(&Scancode::S)
			{
				move_vector -= forward_vector;
			}
			if keyboard_state.contains(&Scancode::D)
			{
				move_vector += left_vector;
			}
			if keyboard_state.contains(&Scancode::A)
			{
				move_vector -= left_vector;
			}

			let move_vector_length = move_vector.magnitude();
			if move_vector_length > 0.0
			{
				move_vector = move_vector / move_vector_length;
			}

			match &mut player_controller.position_source
			{
				PlayerPositionSource::Noclip(position) =>
				{
					let speed = 256.0;
					let jump_speed = 0.8 * speed;

					*position += move_vector * (time_delta_s * speed);

					if keyboard_state.contains(&Scancode::Space)
					{
						position.z += time_delta_s * jump_speed;
					}
					if keyboard_state.contains(&Scancode::C)
					{
						position.z -= time_delta_s * jump_speed;
					}
				},
				PlayerPositionSource::Phys(phys_handle) =>
				{
					let ground_acceleration = 2048.0;
					let air_acceleration = 512.0;
					let max_velocity = 400.0;
					let jump_velocity_add = 256.0;

					let cur_velocity = self.physics.get_object_velocity(*phys_handle);
					let on_ground = self.physics.is_object_on_ground(*phys_handle);

					let acceleration: f32 = if on_ground
					{
						ground_acceleration
					}
					else
					{
						air_acceleration
					};

					let mut velocity_add = Vec3f::zero();

					// Limit maximum velocity.
					let velocity_projection_to_move_vector = move_vector.dot(cur_velocity);
					if velocity_projection_to_move_vector < max_velocity
					{
						let max_can_add = max_velocity - velocity_projection_to_move_vector;
						velocity_add = move_vector * (acceleration * time_delta_s).min(max_can_add);
					}

					if keyboard_state.contains(&Scancode::Space) && on_ground && cur_velocity.z <= 1.0
					{
						velocity_add.z = jump_velocity_add;
					}

					self.physics.add_object_velocity(*phys_handle, &velocity_add);
				},
			}
		}
	}

	fn update_plates(&mut self, time_delta_s: f32)
	{
		for (_id, (plate_component, activation_component, location_component)) in self.ecs.query_mut::<(
			&mut PlateComponent,
			&mut EntityActivationComponent,
			&mut LocationComponent,
		)>()
		{
			let was_activated = activation_component.activated;
			if activation_component.activated
			{
				activation_component.activated = false;
				plate_component.state = PlateState::TargetUp;
			}

			let wait_time_s = 3.0;

			match &mut plate_component.state
			{
				PlateState::TargetUp =>
				{
					location_component.position.z += time_delta_s * plate_component.speed;
					if location_component.position.z >= plate_component.position_upper.z
					{
						location_component.position.z = plate_component.position_upper.z;

						plate_component.state = PlateState::StayTop {
							down_time_s: self.game_time + wait_time_s,
						};
					}
				},
				PlateState::TargetDown =>
				{
					location_component.position.z -= time_delta_s * plate_component.speed;
					if location_component.position.z <= plate_component.position_lower.z
					{
						location_component.position.z = plate_component.position_lower.z;
					}
				},
				PlateState::StayTop { down_time_s } =>
				{
					// Wait a bit starting from moment when trigger was deactivated.
					if was_activated
					{
						*down_time_s = self.game_time + wait_time_s;
					}
					else if self.game_time >= *down_time_s
					{
						plate_component.state = PlateState::TargetDown;
					}
				},
			}
		}
	}

	fn update_doors(&mut self, time_delta_s: f32)
	{
		// Make prepass - trigger activation of slave doors if master door is activated.
		for (id, (door_component,)) in self.ecs.query::<(&DoorComponent,)>().iter()
		{
			if door_component.slave_doors.is_empty()
			{
				continue;
			}

			let mut q = self.ecs.query_one::<(&mut EntityActivationComponent,)>(id).unwrap();
			let activated = q.get().unwrap().0.activated;
			drop(q);

			if activated
			{
				for slave_door_id in &door_component.slave_doors
				{
					if let Ok(mut q) = self.ecs.query_one::<(&mut EntityActivationComponent,)>(*slave_door_id)
					{
						q.get().unwrap().0.activated = true;
					}
				}
			}
		}

		// Perform actual doors logic.
		for (_id, (door_component, activation_component, location_component)) in self
			.ecs
			.query::<(
				&mut DoorComponent,
				&mut EntityActivationComponent,
				&mut LocationComponent,
			)>()
			.iter()
		{
			let was_activated = activation_component.activated;
			if activation_component.activated
			{
				activation_component.activated = false;
				door_component.state = DoorState::TargetOpened;
			}

			let step = time_delta_s * door_component.speed;

			// TODO - avoid computational errors - interpolate positions based on scalar.
			match &mut door_component.state
			{
				DoorState::TargetOpened =>
				{
					let vec_to = door_component.position_opened - door_component.position_closed;
					let vec_to_len = vec_to.magnitude();
					location_component.position += vec_to * (step / vec_to_len);

					let distance_traveled = (location_component.position - door_component.position_closed).magnitude();
					if distance_traveled >= vec_to_len
					{
						location_component.position = door_component.position_opened;

						door_component.state = DoorState::StayOpened {
							down_time_s: self.game_time + door_component.wait,
						};
					}
				},
				DoorState::TargetClosed =>
				{
					let vec_to = door_component.position_closed - door_component.position_opened;
					let vec_to_len = vec_to.magnitude();
					location_component.position += vec_to * (step / vec_to_len);

					let distance_traveled = (location_component.position - door_component.position_opened).magnitude();
					if distance_traveled >= vec_to_len
					{
						location_component.position = door_component.position_closed;
					}
				},
				DoorState::StayOpened { down_time_s } =>
				{
					// Wait a bit starting from moment when trigger was deactivated.
					if was_activated
					{
						*down_time_s = self.game_time + door_component.wait;
					}
					else if self.game_time >= *down_time_s
					{
						door_component.state = DoorState::TargetClosed;
					}
				},
			}
		}
	}

	fn update_buttons(&mut self, time_delta_s: f32)
	{
		for (_id, (button_component, activation_component, location_component)) in self
			.ecs
			.query::<(
				&mut ButtonComponent,
				&mut EntityActivationComponent,
				&mut LocationComponent,
			)>()
			.iter()
		{
			let was_activated = activation_component.activated;
			if activation_component.activated
			{
				activation_component.activated = false;
				button_component.state = ButtonState::TargetPressed;
			}

			let step = time_delta_s * button_component.speed;

			// TODO - avoid computational errors - interpolate positions based on scalar.
			match &mut button_component.state
			{
				ButtonState::TargetPressed =>
				{
					let vec_to = button_component.position_pressed - button_component.position_released;
					let vec_to_len = vec_to.magnitude();
					location_component.position += vec_to * (step / vec_to_len);

					let distance_traveled =
						(location_component.position - button_component.position_released).magnitude();
					if distance_traveled >= vec_to_len
					{
						location_component.position = button_component.position_pressed;

						button_component.state = ButtonState::StayPressed {
							down_time_s: self.game_time + button_component.wait,
						};
					}
				},
				ButtonState::TargetReleased =>
				{
					let vec_to = button_component.position_released - button_component.position_pressed;
					let vec_to_len = vec_to.magnitude();
					location_component.position += vec_to * (step / vec_to_len);

					let distance_traveled =
						(location_component.position - button_component.position_pressed).magnitude();
					if distance_traveled >= vec_to_len
					{
						location_component.position = button_component.position_released;
					}
				},
				ButtonState::StayPressed { down_time_s } =>
				{
					// Wait a bit starting from moment when trigger was deactivated.
					if was_activated
					{
						*down_time_s = self.game_time + button_component.wait;
					}
					else if self.game_time >= *down_time_s
					{
						button_component.state = ButtonState::TargetReleased;
					}
				},
			}
		}
	}

	fn update_trains(&mut self, time_delta_s: f32)
	{
		for (id, (train_component, activation_component)) in self
			.ecs
			.query::<(&mut TrainComponent, &mut EntityActivationComponent)>()
			.iter()
		{
			loop
			{
				match &mut train_component.state
				{
					TrainState::SearchForInitialPosition =>
					{
						let mut q = self
							.ecs
							.query_one::<(&TargetNameComponent,)>(train_component.target)
							.unwrap();
						let target_name = &q.get().unwrap().0.name;

						for (target_id, (named_target_component,)) in
							self.ecs.query::<(&NamedTargetComponent,)>().iter()
						{
							if named_target_component.name == *target_name
							{
								// Just started. Set location to location of first target.
								let mut dst_q = self.ecs.query_one::<(&LocationComponent,)>(target_id).unwrap();
								let dst_position = dst_q.get().unwrap().0.position;
								drop(dst_q);

								let mut q = self
									.ecs
									.query_one::<(&mut LocationComponent, Option<&NamedTargetComponent>)>(id)
									.unwrap();
								let (location_component, named_target_component) = q.get().unwrap();
								location_component.position = dst_position + train_component.target_shift;

								train_component.target = target_id;

								// Wait for activation if this entity has NamedTargetComponent. Else - start moving immediately.
								train_component.state = if named_target_component.is_some()
								{
									TrainState::WaitForActivation
								}
								else
								{
									TrainState::Move
								};
								break;
							}
						}

						// Continue in order to process move.
						continue;
					},
					TrainState::WaitForActivation =>
					{
						if activation_component.activated
						{
							activation_component.activated = false;
							train_component.state = TrainState::SearchForNextTarget;
							// Continue in order to process move.
							continue;
						}
					},
					TrainState::SearchForNextTarget =>
					{
						let mut q = self
							.ecs
							.query_one::<(&TargetNameComponent,)>(train_component.target)
							.unwrap();
						let target_name = &q.get().unwrap().0.name;

						for (target_id, (named_target_component,)) in
							self.ecs.query::<(&NamedTargetComponent,)>().iter()
						{
							if named_target_component.name == *target_name
							{
								train_component.state = TrainState::Move;
								train_component.target = target_id;
								break;
							}
						}

						// Continue in order to process move.
						continue;
					},
					TrainState::Move =>
					{
						let mut dst_q = self
							.ecs
							.query_one::<(&LocationComponent, &WaitComponent)>(train_component.target)
							.unwrap();
						let (dst_location_component, dst_wait_component) = dst_q.get().unwrap();
						let dst_position = dst_location_component.position + train_component.target_shift;
						let dst_wait = dst_wait_component.wait;
						drop(dst_q);

						let mut q = self.ecs.query_one::<(&mut LocationComponent,)>(id).unwrap();
						let position = &mut q.get().unwrap().0.position;

						let vec_to = dst_position - *position;
						let vec_to_len = vec_to.magnitude();
						if vec_to_len != 0.0
						{
							let step = time_delta_s * train_component.speed;
							if vec_to_len > step
							{
								*position += vec_to * (step / vec_to_len);
							}
							else
							{
								*position = dst_position;
							}
						}
						else
						{
							*position = dst_position;
						}

						if *position == dst_position
						{
							if dst_wait < 0.0
							{
								// Wait forever.
								train_component.state = TrainState::Wait {
									continue_time_s: self.game_time + 1.0e12,
								};
							}
							else if dst_wait > 0.0
							{
								train_component.state = TrainState::Wait {
									continue_time_s: self.game_time + dst_wait,
								};
							}
							else
							{
								train_component.state = TrainState::SearchForNextTarget;
							}
						}
					},
					TrainState::Wait { continue_time_s } =>
					{
						if self.game_time >= *continue_time_s
						{
							train_component.state = TrainState::SearchForNextTarget;
						}
					},
				}

				// Normally break (if we do not need to perform state transition).
				break;
			}
		}
	}

	fn update_test_submodels(&mut self)
	{
		for (_id, (test_submodel_component, location_component)) in
			self.ecs.query_mut::<(&TestSubmodelComponent, &mut LocationComponent)>()
		{
			let index = test_submodel_component.index;
			let phase = index as f32;
			let shift = 32.0 *
				Vec3f::new(
					(0.5 * self.game_time + phase).sin(),
					(0.33 * self.game_time + phase).sin(),
					(0.11111 * self.game_time + phase).sin(),
				);

			let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

			location_component.position = bbox.get_center() + shift;
		}
		self.ecs_command_buffer.run_on(&mut self.ecs);
	}

	fn update_kinematic_physics_objects(&mut self)
	{
		for (_id, (location_kinematic_physics_object_component, location_component)) in self
			.ecs
			.query_mut::<(&LocationKinematicPhysicsObjectComponent, &LocationComponent)>()
		{
			self.physics.set_kinematic_object_position(
				location_kinematic_physics_object_component.phys_handle,
				&location_component.position,
			);
		}
	}

	fn update_touch_triggers(&mut self)
	{
		for (_id, (touch_trigger_component, trigger_single_target_component, target_name_component)) in self
			.ecs
			.query::<(
				&TouchTriggerComponent,
				Option<&TriggerSingleTargetComponent>,
				Option<&TargetNameComponent>,
			)>()
			.iter()
		{
			self.physics
				.get_box_touching_entities(&touch_trigger_component.bbox, |entity| {
					// Only player can activate triggers.
					if let Ok(mut q) = self.ecs.query_one::<(&mut PlayerComponent,)>(entity)
					{
						if let Some((_player_component,)) = q.get()
						{
						}
						else
						{
							return;
						}
					}
					else
					{
						return;
					}

					// Activate target.
					if let Some(t) = trigger_single_target_component
					{
						if let Ok(mut q) = self.ecs.query_one::<(&mut EntityActivationComponent,)>(t.target)
						{
							if let Some((entity_activation_component,)) = q.get()
							{
								entity_activation_component.activated = true;
							}
						}
					}
					// Activate named targets.
					if let Some(TargetNameComponent { name }) = target_name_component
					{
						for (target_id, (named_target_component,)) in
							self.ecs.query::<(&NamedTargetComponent,)>().iter()
						{
							if named_target_component.name == *name
							{
								if let Ok(mut q) = self.ecs.query_one::<(&mut EntityActivationComponent,)>(target_id)
								{
									if let Some((actication_component,)) = q.get()
									{
										actication_component.activated = true;
									}
								}
							}
						}
					}
				});
		}
	}

	fn update_named_activations(&mut self)
	{
		for (id, (target_name_component,)) in self.ecs.query::<(&TargetNameComponent,)>().iter()
		{
			let mut activated = false;
			if let Ok(mut q) = self.ecs.query_one::<(&EntityActivationComponent,)>(id)
			{
				if let Some((actication_component,)) = q.get()
				{
					activated = actication_component.activated;
				}
			}

			if activated
			{
				for (target_id, (named_target_component,)) in self.ecs.query::<(&NamedTargetComponent,)>().iter()
				{
					if named_target_component.name == target_name_component.name
					{
						if let Ok(mut q) = self.ecs.query_one::<(&mut EntityActivationComponent,)>(target_id)
						{
							if let Some((actication_component,)) = q.get()
							{
								actication_component.activated = true;
							}
						}
					}
				}
			}
		}
	}

	fn update_animations(&mut self)
	{
		for (_id, (_simple_animation_component, model)) in
			self.ecs.query_mut::<(&SimpleAnimationComponent, &mut ModelEntity)>()
		{
			let num_frames = model.model.frames_info.len() as u32;
			let frame_f = self.game_time * 10.0;
			model.animation.frames[0] = (frame_f as u32) % num_frames;
			model.animation.frames[1] = (frame_f as u32 + 1) % num_frames;
			model.animation.lerp = 1.0 - frame_f.fract();
		}
	}

	fn update_player_controller_locations(&mut self)
	{
		for (_id, (_player_controller_location_component, player_controller, location)) in self.ecs.query_mut::<(
			&PlayerControllerLocationComponent,
			&PlayerControllerComponent,
			&mut LocationComponent,
		)>()
		{
			location.position = match player_controller.position_source
			{
				PlayerPositionSource::Noclip(p) => p,
				PlayerPositionSource::Phys(handle) => self.physics.get_object_location(handle).0,
			};

			location.rotation = player_controller.rotation_controller.get_rotation();
		}
	}

	fn update_phys_model_locations(&mut self)
	{
		for (_id, (phys_handle, location)) in self
			.ecs
			.query_mut::<(&PhysicsLocationComponent, &mut LocationComponent)>()
		{
			let phys_location = self.physics.get_object_location(*phys_handle);
			location.position = phys_location.0;
			location.rotation = phys_location.1;
		}
	}

	fn update_other_entity_locations(&mut self)
	{
		for (_id, (other_entity_location_component, location_component)) in self
			.ecs
			.query::<(&OtherEntityLocationComponent, &mut LocationComponent)>()
			.into_iter()
		{
			// TODO - support chains of linked entities.
			let mut q = self
				.ecs
				.query_one::<(&LocationComponent,)>(other_entity_location_component.entity)
				.unwrap();
			let (src_location_component,) = q.get().unwrap();

			location_component.position = src_location_component.position +
				src_location_component
					.rotation
					.rotate_vector(other_entity_location_component.relative_position);
			location_component.rotation =
				src_location_component.rotation * other_entity_location_component.relative_rotation;
		}
	}

	fn update_models_locations(&mut self)
	{
		for (_id, (_model_entity_location_link_component, location_component, model)) in
			self.ecs
				.query_mut::<(&ModelEntityLocationLinkComponent, &LocationComponent, &mut ModelEntity)>()
		{
			model.position = location_component.position;
			model.rotation = location_component.rotation;
		}
	}

	fn update_submodels_locations(&mut self)
	{
		for (_id, (_submodel_entity_with_index_location_component, location_component, submodel_entity_with_index)) in
			self.ecs.query_mut::<(
				&SubmodelEntityWithIndexLocationLinkComponent,
				&LocationComponent,
				&mut SubmodelEntityWithIndex,
			)>()
		{
			submodel_entity_with_index.submodel_entity.position = location_component.position;
			submodel_entity_with_index.submodel_entity.rotation = location_component.rotation;
		}
	}

	fn collect_drawable_components<T: hecs::Component + Clone>(&self) -> Vec<T>
	{
		self.ecs.query::<(&T,)>().iter().map(|(_id, (c,))| c.clone()).collect()
	}

	fn get_camera_location(&self) -> (Vec3f, QuaternionF)
	{
		let mut q = self.ecs.query_one::<(&LocationComponent,)>(self.player_entity).unwrap();
		let (location_component,) = q.get().unwrap();

		(location_component.position, location_component.rotation)
	}

	fn get_camera_angles(&self) -> (f32, f32, f32)
	{
		let mut q = self
			.ecs
			.query_one::<(&PlayerControllerComponent,)>(self.player_entity)
			.unwrap();
		let (player_controller,) = q.get().unwrap();

		player_controller.rotation_controller.get_angles()
	}

	fn set_camera_angles(&mut self, azimuth: f32, elevation: f32, roll: f32)
	{
		let mut q = self
			.ecs
			.query_one::<(&mut PlayerControllerComponent,)>(self.player_entity)
			.unwrap();
		let (player_controller,) = q.get().unwrap();

		player_controller
			.rotation_controller
			.set_angles(azimuth, elevation, roll)
	}

	fn process_commands(&mut self)
	{
		let queue_ptr_copy = self.commands_queue.clone();
		queue_ptr_copy.lock().unwrap().process_commands(self);
	}

	fn command_get_pos(&mut self, _args: commands_queue::CommandArgs)
	{
		let pos = self.get_camera_location().0;
		self.console
			.lock()
			.unwrap()
			.add_text(format!("{} {} {}", pos.x, pos.y, pos.z));
	}

	fn command_set_pos(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console.lock().unwrap().add_text("Expected 3 args".to_string());
			return;
		}

		if let (Ok(x), Ok(y), Ok(z)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			let pos = Vec3f::new(x, y, z);
			let mut q = self
				.ecs
				.query_one::<(&mut PlayerControllerComponent,)>(self.player_entity)
				.unwrap();
			let (player_controller,) = q.get().unwrap();

			match &mut player_controller.position_source
			{
				PlayerPositionSource::Noclip(dst_pos) =>
				{
					*dst_pos = pos;
				},
				PlayerPositionSource::Phys(phys_handle) =>
				{
					self.physics.teleport_object(*phys_handle, &pos);
				},
			}
		}
		else
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Failed to parse args".to_string());
		}
	}

	fn command_get_angles(&mut self, _args: commands_queue::CommandArgs)
	{
		let angles = self.get_camera_angles();
		self.console
			.lock()
			.unwrap()
			.add_text(format!("{} {} {}", angles.0, angles.1, angles.2));
	}

	fn command_set_angles(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.lock().unwrap().add_text("Expected 2 args".to_string());
			return;
		}

		let azimuth = args[0].parse::<f32>();
		let elevation = args[1].parse::<f32>();
		let roll = if args.len() > 2
		{
			args[2].parse::<f32>().unwrap_or(0.0)
		}
		else
		{
			0.0
		};

		if let (Ok(azimuth), Ok(elevation)) = (azimuth, elevation)
		{
			self.set_camera_angles(azimuth, elevation, roll);
		}
		else
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Failed to parse args".to_string());
		}
	}

	fn command_add_test_light(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console.lock().unwrap().add_text("Expected 3 args".to_string());
			return;
		}

		if let (Ok(r), Ok(g), Ok(b)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			self.ecs.spawn((
				TestLightComponent {},
				PointLight {
					pos: self.get_camera_location().0,
					color: [r * 1024.0, g * 1024.0, b * 1024.0],
				},
			));
		}
		else
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Failed to parse args".to_string());
		}
	}

	fn command_reset_test_lights(&mut self, _args: commands_queue::CommandArgs)
	{
		for (id, (_test_light_component,)) in self.ecs.query_mut::<(&TestLightComponent,)>()
		{
			self.ecs_command_buffer.despawn(id);
		}
		self.ecs_command_buffer.run_on(&mut self.ecs);
	}

	fn command_add_test_model(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.lock().unwrap().add_text("Expected 2 args".to_string());
			return;
		}

		let mut r = self.resources_manager.lock().unwrap();
		let model = r.get_model(&args[0]);
		let texture = r.get_texture_lite(&args[1]);

		let (position, rotation) = self.get_camera_location();
		let bbox = model.frames_info[0].bbox;

		let entity = self.ecs.spawn((
			TestModelComponent {},
			SimpleAnimationComponent {},
			LocationComponent { position, rotation },
			ModelEntityLocationLinkComponent {},
			ModelEntity {
				position,
				rotation,
				animation: AnimationPoint {
					frames: [0, 0],
					lerp: 0.0,
				},
				model,
				texture,
				blending_mode: material::BlendingMode::None,
				lighting: ModelLighting::Default,
				is_view_model: false,
				ordering_custom_bbox: None,
			},
		));
		self.ecs
			.insert_one(entity, self.physics.add_object(entity, &position, &rotation, &bbox))
			.ok();
	}

	fn command_reset_test_models(&mut self, _args: commands_queue::CommandArgs)
	{
		for (id, (_test_model_component, phys_handle)) in self
			.ecs
			.query_mut::<(&TestModelComponent, &test_game_physics::ObjectHandle)>()
		{
			self.physics.remove_object(*phys_handle);
			self.ecs_command_buffer.despawn(id);
		}
		self.ecs_command_buffer.run_on(&mut self.ecs);
	}

	fn command_add_test_decal(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 1
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Expected at least 1 arg".to_string());
			return;
		}

		let texture = self.resources_manager.lock().unwrap().get_texture_lite(&args[0]);
		let scale = if args.len() >= 2
		{
			args[1].parse::<f32>().unwrap_or(1.0)
		}
		else
		{
			1.0
		};

		let texture_mip0 = &texture[0];
		let size = Vec3f::new(
			texture_mip0.size[0].min(texture_mip0.size[1]) as f32,
			texture_mip0.size[0] as f32,
			texture_mip0.size[1] as f32,
		) * (0.5 * scale);

		let (position, rotation) = self.get_camera_location();

		self.ecs.spawn((
			TestDecalComponent {},
			Decal {
				position,
				rotation,
				scale: size,
				texture,
				blending_mode: material::BlendingMode::None,
				lightmap_light_scale: 1.0,
				light_add: [0.0; 3],
			},
		));
	}

	fn command_reset_test_decals(&mut self, _args: commands_queue::CommandArgs)
	{
		for (id, (_test_decal_component,)) in self.ecs.query_mut::<(&TestDecalComponent,)>()
		{
			self.ecs_command_buffer.despawn(id)
		}
		self.ecs_command_buffer.run_on(&mut self.ecs);
	}

	fn command_set_view_model(&mut self, args: commands_queue::CommandArgs)
	{
		self.command_reset_view_model(Vec::new());

		if args.len() < 2
		{
			self.console.lock().unwrap().add_text("Expected 2 args".to_string());
			return;
		}

		let mut r = self.resources_manager.lock().unwrap();
		let model = r.get_model(&args[0]);
		let texture = r.get_texture_lite(&args[1]);

		let position = Vec3f::zero();
		let rotation = QuaternionF::zero();

		let view_model_entity = self.ecs.spawn((
			SimpleAnimationComponent {},
			OtherEntityLocationComponent {
				entity: self.player_entity,
				relative_position: Vec3f::new(16.0, -8.0, -10.0),
				relative_rotation: QuaternionF::from_angle_x(Rad(0.0)),
			},
			LocationComponent { position, rotation },
			ModelEntityLocationLinkComponent {},
			ModelEntity {
				position,
				rotation,
				animation: AnimationPoint {
					frames: [0, 0],
					lerp: 0.0,
				},
				model,
				texture,
				blending_mode: material::BlendingMode::Average,
				lighting: ModelLighting::Default,
				is_view_model: true,
				ordering_custom_bbox: None,
			},
		));

		let mut q = self
			.ecs
			.query_one::<(&mut PlayerComponent,)>(self.player_entity)
			.unwrap();
		let (player_component,) = q.get().unwrap();
		player_component.view_model_entity = view_model_entity;
	}

	fn command_reset_view_model(&mut self, _args: commands_queue::CommandArgs)
	{
		let mut q = self
			.ecs
			.query_one::<(&mut PlayerComponent,)>(self.player_entity)
			.unwrap();
		let (player_component,) = q.get().unwrap();

		if player_component.view_model_entity != hecs::Entity::DANGLING
		{
			let view_model_entity = player_component.view_model_entity;
			player_component.view_model_entity = hecs::Entity::DANGLING;
			drop(q);
			let _ignore = self.ecs.despawn(view_model_entity);
		}
	}

	fn command_noclip(&mut self, _args: commands_queue::CommandArgs)
	{
		let mut q = self
			.ecs
			.query_one::<(&mut PlayerControllerComponent,)>(self.player_entity)
			.unwrap();
		let (player_controller,) = q.get().unwrap();

		let new_position_source = match player_controller.position_source
		{
			PlayerPositionSource::Noclip(pos) =>
			{
				self.console.lock().unwrap().add_text("Noclip OFF".to_string());

				PlayerPositionSource::Phys(self.physics.add_character_object(self.player_entity, &pos, 60.0, 120.0))
			},
			PlayerPositionSource::Phys(phys_handle) =>
			{
				self.console.lock().unwrap().add_text("Noclip ON".to_string());

				let pos = self.physics.get_object_location(phys_handle).0;
				self.physics.remove_object(phys_handle);

				PlayerPositionSource::Noclip(pos)
			},
		};

		player_controller.position_source = new_position_source;
	}
}

impl GameInterface for Game
{
	fn update(
		&mut self,
		keyboard_state: &system_window::KeyboardState,
		events: &[sdl2::event::Event],
		time_delta_s: f32,
	)
	{
		self.process_commands();

		self.game_time += time_delta_s;

		self.update_player_entity(keyboard_state, events, time_delta_s);
		self.update_test_submodels();
		self.update_plates(time_delta_s);
		self.update_doors(time_delta_s);
		self.update_buttons(time_delta_s);
		self.update_trains(time_delta_s);
		self.update_kinematic_physics_objects();

		// Update physics only after settig params of externally-controlled physics objects - player, platforms, etc.
		self.physics.update(time_delta_s);

		// Update models after physics update in order to setup position properly.

		self.update_touch_triggers();
		self.update_named_activations();

		self.update_animations();

		// Update location of player entity, taken from player controller.
		self.update_player_controller_locations();
		// Take locations from physics engine.
		self.update_phys_model_locations();
		// Take locations from other entities. This is needed for entities, attached to other entities.
		self.update_other_entity_locations();

		// Update locations of visible models.
		self.update_models_locations();
		self.update_submodels_locations();
	}

	fn grab_mouse_input(&self) -> bool
	{
		true
	}

	fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> FrameInfo
	{
		let (pos, angles) = self.get_camera_location();

		let fov = std::f32::consts::PI * 0.375;
		let camera_matrices = build_view_matrix_with_full_rotation(
			pos,
			angles,
			fov,
			surface_info.width as f32,
			surface_info.height as f32,
		);

		let mut submodel_entities = vec![None; self.map.submodels.len()];
		for (_id, (submodel_entity_with_index,)) in self.ecs.query::<(&SubmodelEntityWithIndex,)>().iter()
		{
			submodel_entities[submodel_entity_with_index.index] = Some(submodel_entity_with_index.submodel_entity);
		}

		FrameInfo {
			camera_matrices,
			submodel_entities,
			skybox_rotation: QuaternionF::zero(),
			game_time_s: self.game_time,
			lights: self.collect_drawable_components(),
			model_entities: self.collect_drawable_components(),
			decals: self.collect_drawable_components(),
		}
	}

	fn draw_frame_overlay(&self, pixels: &mut [Color32], surface_info: &system_window::SurfaceInfo)
	{
		let center_x = surface_info.width / 2;
		let center_y = surface_info.height / 2;
		let half_length = 12;
		let half_width = 1;
		for y in center_y - half_width ..= center_y + half_width
		{
			for x in center_x - half_length ..= center_x + half_length
			{
				let dst = &mut pixels[x + y * surface_info.pitch];
				*dst = dst.get_inverted();
			}
		}

		for y in center_y - half_length ..= center_y + half_length
		{
			for x in center_x - half_width ..= center_x + half_width
			{
				let dst = &mut pixels[x + y * surface_info.pitch];
				*dst = dst.get_inverted();
			}
		}
	}
}

// Returns normalized vector.
fn get_entity_move_direction(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> Vec3f
{
	if let Some(angle) = get_entity_key_value(entity, map, "angle")
	{
		if let Ok(num) = angle.parse::<f32>()
		{
			if num == -1.0
			{
				return Vec3f::new(0.0, 0.0, 1.0);
			}
			if num == -2.0
			{
				return Vec3f::new(0.0, 0.0, -1.0);
			}
			let angle_rad = num * (std::f32::consts::PI / 180.0);
			return Vec3f::new(angle_rad.cos(), angle_rad.sin(), 0.0);
		}
	}

	// TODO - support "angles".

	// Return something.
	return Vec3f::unit_x();
}

fn get_entity_f32(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap, key: &str) -> Option<f32>
{
	get_entity_key_value(entity, map, key).unwrap_or("").parse::<f32>().ok()
}

fn get_entity_classname<'a>(entity: &bsp_map_compact::Entity, map: &'a bsp_map_compact::BSPMap) -> Option<&'a str>
{
	get_entity_key_value(entity, map, "classname")
}

fn get_entity_key_value<'a>(
	entity: &bsp_map_compact::Entity,
	map: &'a bsp_map_compact::BSPMap,
	key: &str,
) -> Option<&'a str>
{
	for key_value in &map.key_value_pairs
		[entity.first_key_value_pair as usize .. (entity.first_key_value_pair + entity.num_key_value_pairs) as usize]
	{
		let actual_key = bsp_map_compact::get_map_string(key_value.key, map);
		if actual_key == key
		{
			return Some(bsp_map_compact::get_map_string(key_value.value, map));
		}
	}

	None
}

impl Drop for Game
{
	fn drop(&mut self)
	{
		// HACK! Save command queue pointer casted to "dyn" in order to avoid nasty bug with broken identity of dynamic objects.
		// See https://github.com/rust-lang/rust/issues/46139.
		let commands_processor = self.commands_processor.clone();
		commands_processor
			.lock()
			.unwrap()
			.remove_command_queue(&self.commands_queue_dyn);
	}
}
