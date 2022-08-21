use super::{
	commands_processor, commands_queue, components::*, console, frame_info::*, game_interface::*, light::*,
	resources_manager::*, test_game_physics,
};
use square_wheel_lib::common::{
	bbox::*, bsp_map_compact, camera_rotation_controller::*, color::*, material, math_types::*, matrix::*,
	system_window,
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
					}
				},
				Some("trigger_multiple") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						// Spawn trigger.
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);

						let entity = self.ecs.spawn(());
						self.ecs.insert_one(entity, TriggerComponent { bbox }).ok();

						// Test visualiation.
						if false
						{
							self.ecs
								.insert_one(
									entity,
									SubmodelEntityWithIndex {
										index,
										submodel_entity: SubmodelEntity {
											position: bbox.get_center(),
											rotation: QuaternionF::zero(),
										},
									},
								)
								.ok();
						}
					}
				},
				Some("func_plat") =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						let bbox = bsp_map_compact::get_submodel_bbox(&self.map, &self.map.submodels[index]);
						let height = bbox.max.z - bbox.min.z;

						let position_upper = bbox.get_center();
						let position_lower = position_upper - Vec3f::new(0.0, 0.0, height);

						let position = position_lower;
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
									PlateComponent {
										phys_handle: self.physics.add_submodel_object(
											entity,
											index,
											&Vec3f::new(0.0, 0.0, -height),
											&rotation,
										),
										speed: get_entity_key_value(map_entity, &self.map, "speed")
											.unwrap_or("")
											.parse::<f32>()
											.unwrap_or(150.0),
										position_lower,
										position_upper,
										state: PlateState::TargetDown,
									},
								),
							)
							.ok();

						// Add activation trigger.
						let bbox_half_size = bbox.get_size() * 0.5;
						let bbox_reduce_min = Vec3f::new(
							(bbox_half_size.x - 1.0).min(25.0),
							(bbox_half_size.y - 1.0).min(25.0),
							0.0,
						);
						let bbox_reduce_max = Vec3f::new(
							(bbox_half_size.x - 1.0).min(25.0),
							(bbox_half_size.y - 1.0).min(25.0),
							(bbox_half_size.z - 1.0).min(-8.0),
						);

						let trigger_bbox = BBox::from_min_max(bbox.min + bbox_reduce_min, bbox.max - bbox_reduce_max);

						self.ecs.spawn((
							TriggerComponent { bbox: trigger_bbox },
							TrggerSingleTargetComponent { target: entity },
						));
					}
				},
				_ =>
				{
					let index = map_entity.submodel_index as usize;
					if index < self.map.submodels.len()
					{
						// Spawn test submodel.
						let entity = self.ecs.spawn(());
						self.ecs
							.insert_one(
								entity,
								TestSubmodelComponent {
									phys_handle: self.physics.add_submodel_object(
										entity,
										index,
										&Vec3f::zero(),
										&QuaternionF::zero(),
									),
									index,
								},
							)
							.ok();
					}
				},
			}
		}

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
		for (_id, (plate_component, activation_component, location_component, submodel_entity_with_index)) in
			self.ecs.query_mut::<(
				&mut PlateComponent,
				&mut EntityActivationComponent,
				&mut LocationComponent,
				&mut SubmodelEntityWithIndex,
			)>()
		{
			let was_activated = activation_component.activated;
			if activation_component.activated
			{
				activation_component.activated = false;
				plate_component.state = PlateState::TargetUp;
			}

			match plate_component.state
			{
				PlateState::TargetUp =>
				{
					location_component.position.z += time_delta_s * plate_component.speed;
					if location_component.position.z >= plate_component.position_upper.z
					{
						if !was_activated
						{
							// Start moving down when reached upper position, but wait until activation is active.
							plate_component.state = PlateState::TargetDown;
						}
						location_component.position.z = plate_component.position_upper.z;
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
			}

			self.physics
				.set_kinematic_object_position(plate_component.phys_handle, &location_component.position);

			submodel_entity_with_index.submodel_entity.position = location_component.position;
		}
	}

	fn update_test_submodels(&mut self)
	{
		for (id, (test_submodel_component,)) in self.ecs.query::<(&TestSubmodelComponent,)>().iter()
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
			let bbox_center = bbox.get_center();

			self.physics
				.set_kinematic_object_position(test_submodel_component.phys_handle, &(bbox_center + shift));
			let (position, rotation) = self.physics.get_object_location(test_submodel_component.phys_handle);

			if let Ok(mut q) = self.ecs.query_one::<(&mut SubmodelEntityWithIndex,)>(id)
			{
				if let Some((submodel_entity_with_index,)) = q.get()
				{
					submodel_entity_with_index.submodel_entity.position = position;
					submodel_entity_with_index.submodel_entity.rotation = rotation;
				}
				else
				{
					self.ecs_command_buffer.insert_one(
						id,
						SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity { position, rotation },
						},
					);
				}
			}
		}
		self.ecs_command_buffer.run_on(&mut self.ecs);
	}

	fn update_triggers(&mut self)
	{
		for (_id, (trigger_component, trigger_single_target_component)) in self
			.ecs
			.query::<(&TriggerComponent, Option<&TrggerSingleTargetComponent>)>()
			.iter()
		{
			self.physics
				.get_box_touching_entities(&trigger_component.bbox, |entity| {
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
				});
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

		// Update physics only after settig params of externally-controlled physics objects - player, platforms, etc.
		self.physics.update(time_delta_s);

		// Update models after physics update in order to setup position properly.

		self.update_triggers();

		self.update_animations();

		// Update location of player entity, taken from player controller.
		self.update_player_controller_locations();
		// Take locations from physics engine.
		self.update_phys_model_locations();
		// Take locations from other entities. This is needed for entities, attached to other entities.
		self.update_other_entity_locations();

		// Update locations of visible models.
		self.update_models_locations();
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
