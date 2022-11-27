use super::{components::*, frame_info::*, resources_manager::*, test_game_physics::*};
use square_wheel_lib::common::{material, math_types::*, system_window};

pub fn update_player_entity(
	ecs: &mut hecs::World,
	physics: &mut TestGamePhysics,
	resources_manager: &mut ResourcesManager,
	player_entity: hecs::Entity,
	keyboard_state: &system_window::KeyboardState,
	events: &[sdl2::event::Event],
	game_time: f32,
	time_delta_s: f32,
)
{
	let mut q = if let Ok(q) = ecs.query_one::<(&PlayerComponent, &mut PlayerControllerComponent)>(player_entity)
	{
		q
	}
	else
	{
		return;
	};

	let (player_component, player_controller) = q.get().unwrap();
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

			let cur_velocity = physics.get_object_velocity(*phys_handle);
			let on_ground = physics.is_object_on_ground(*phys_handle);

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

			physics.add_object_velocity(*phys_handle, &velocity_add);
		},
	}

	let player_position = match player_controller.position_source
	{
		PlayerPositionSource::Noclip(p) => p,
		PlayerPositionSource::Phys(handle) => physics.get_object_location(handle).0,
	};

	let camera_rotation = player_controller.rotation_controller.get_rotation();

	let mut flashlight_entity = player_component.flashlight_entity;

	drop(q);

	let mut has_mouse_down = false;
	let mut has_flashlight_toggle = false;
	for event in events
	{
		match event
		{
			sdl2::event::Event::MouseButtonDown { .. } =>
			{
				has_mouse_down = true;
			},
			sdl2::event::Event::KeyDown {
				scancode: Some(sdl2::keyboard::Scancode::F),
				..
			} =>
			{
				has_flashlight_toggle = true;
			},
			_ =>
			{},
		}
	}

	if has_mouse_down
	{
		let position = player_position;
		let camera_vector = camera_rotation.rotate_vector(Vec3f::unit_x());

		ecs.spawn((
			LocationComponent {
				position: player_position,
				rotation: QuaternionF::one(),
			},
			DynamicLightLocationLinkComponent {},
			DynamicLight {
				position,
				radius: 128.0,
				color: [32.0 * 1024.0, 64.0 * 1024.0, 32.0 * 1024.0],
				shadow_type: DynamicLightShadowType::Cubemap,
			},
			TimedDespawnComponent {
				despawn_time: game_time + 5.0,
			},
			TestProjectileComponent {
				velocity: camera_vector * 256.0,
				angular_velocity: 5.0,
			},
			Sprite {
				position,
				angle: 0.0,
				radius: 32.0,
				texture: resources_manager.get_texture_lite("shot_sprite.png"),
				orientation: SpriteOrientation::FacingTowardsCamera,
				blending_mode: material::BlendingMode::AlphaBlend,
				light_scale: 0.0,
				light_add: [32.0, 64.0, 32.0],
			},
			SpriteLocationLinkComponent {},
		));
	}

	if has_flashlight_toggle
	{
		if flashlight_entity == hecs::Entity::DANGLING
		{
			let position = player_position;
			let rotation = camera_rotation;

			let brightness = 128.0 * 1024.0;

			flashlight_entity = ecs.spawn((
				LocationComponent { position, rotation },
				PlayerControllerCameraLocationComponent {
					entity: player_entity,
					camera_view_offset: Vec3f::new(0.0, 0.0, 20.0),
					relative_position: Vec3f::new(0.0, 12.0, -15.0),
					relative_rotation: QuaternionF::from_angle_z(Rad(-0.02 * std::f32::consts::PI)),
				},
				DynamicLightLocationLinkComponent {},
				DynamicLight {
					position,
					radius: 512.0,
					color: [brightness, brightness, brightness],
					shadow_type: DynamicLightShadowType::Projector {
						rotation,
						fov: Rad(0.2 * std::f32::consts::PI),
					},
				},
			));
		}
		else
		{
			ecs.despawn(flashlight_entity).ok();
			flashlight_entity = hecs::Entity::DANGLING;
		}

		let mut q = if let Ok(q) = ecs.query_one::<(&mut PlayerComponent,)>(player_entity)
		{
			q
		}
		else
		{
			return;
		};

		q.get().unwrap().0.flashlight_entity = flashlight_entity;
	}
}

pub fn despawn_timed_entites(ecs: &mut hecs::World, ecs_command_buffer: &mut hecs::CommandBuffer, game_time: f32)
{
	for (id, (timed_despawn_component,)) in ecs.query_mut::<(&TimedDespawnComponent,)>()
	{
		if game_time >= timed_despawn_component.despawn_time
		{
			// TODO - free external resources here?
			ecs_command_buffer.despawn(id);
		}
	}

	ecs_command_buffer.run_on(ecs);
}

pub fn update_test_projectiles(ecs: &mut hecs::World, time_delta_s: f32)
{
	for (_id, (test_projectile_component, location_component, sprite_component)) in
		ecs.query_mut::<(&TestProjectileComponent, &mut LocationComponent, Option<&mut Sprite>)>()
	{
		location_component.position += test_projectile_component.velocity * time_delta_s;
		if let Some(sprite_component) = sprite_component
		{
			sprite_component.angle += test_projectile_component.angular_velocity * time_delta_s;
		}
	}
}

pub fn update_plates(ecs: &mut hecs::World, game_time: f32, time_delta_s: f32)
{
	for (_id, (plate_component, activation_component, location_component)) in ecs.query_mut::<(
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
				location_component.position = move_towards_target(
					&location_component.position,
					&plate_component.position_upper,
					plate_component.speed,
					time_delta_s,
				);
				if location_component.position == plate_component.position_upper
				{
					plate_component.state = PlateState::StayTop {
						down_time_s: game_time + wait_time_s,
					};
				}
			},
			PlateState::TargetDown =>
			{
				location_component.position = move_towards_target(
					&location_component.position,
					&plate_component.position_lower,
					plate_component.speed,
					time_delta_s,
				);
			},
			PlateState::StayTop { down_time_s } =>
			{
				// Wait a bit starting from moment when trigger was deactivated.
				if was_activated
				{
					*down_time_s = game_time + wait_time_s;
				}
				else if game_time >= *down_time_s
				{
					plate_component.state = PlateState::TargetDown;
				}
			},
		}
	}
}

pub fn update_doors(ecs: &mut hecs::World, game_time: f32, time_delta_s: f32)
{
	// Make prepass - trigger activation of slave doors if master door is activated.
	for (id, (door_component,)) in ecs.query::<(&DoorComponent,)>().iter()
	{
		if door_component.slave_doors.is_empty()
		{
			continue;
		}

		let mut q = ecs.query_one::<(&mut EntityActivationComponent,)>(id).unwrap();
		let activated = q.get().unwrap().0.activated;
		drop(q);

		if activated
		{
			for slave_door_id in &door_component.slave_doors
			{
				if let Ok(mut q) = ecs.query_one::<(&mut EntityActivationComponent,)>(*slave_door_id)
				{
					q.get().unwrap().0.activated = true;
				}
			}
		}
	}

	// Perform actual doors logic.
	for (_id, (door_component, activation_component, location_component)) in ecs
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

		match &mut door_component.state
		{
			DoorState::TargetOpened =>
			{
				location_component.position = move_towards_target(
					&location_component.position,
					&door_component.position_opened,
					door_component.speed,
					time_delta_s,
				);
				if location_component.position == door_component.position_opened
				{
					door_component.state = DoorState::StayOpened {
						down_time_s: game_time + door_component.wait,
					};
				}
			},
			DoorState::TargetClosed =>
			{
				location_component.position = move_towards_target(
					&location_component.position,
					&door_component.position_closed,
					door_component.speed,
					time_delta_s,
				);
			},
			DoorState::StayOpened { down_time_s } =>
			{
				// Wait a bit starting from moment when trigger was deactivated.
				if was_activated
				{
					*down_time_s = game_time + door_component.wait;
				}
				else if game_time >= *down_time_s
				{
					door_component.state = DoorState::TargetClosed;
				}
			},
		}
	}
}

pub fn update_buttons(ecs: &mut hecs::World, game_time: f32, time_delta_s: f32)
{
	for (_id, (button_component, activation_component, location_component)) in ecs
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

		match &mut button_component.state
		{
			ButtonState::TargetPressed =>
			{
				location_component.position = move_towards_target(
					&location_component.position,
					&button_component.position_pressed,
					button_component.speed,
					time_delta_s,
				);
				if location_component.position == button_component.position_pressed
				{
					button_component.state = ButtonState::StayPressed {
						down_time_s: game_time + button_component.wait,
					};
				}
			},
			ButtonState::TargetReleased =>
			{
				location_component.position = move_towards_target(
					&location_component.position,
					&button_component.position_released,
					button_component.speed,
					time_delta_s,
				);
			},
			ButtonState::StayPressed { down_time_s } =>
			{
				// Wait a bit starting from moment when trigger was deactivated.
				if was_activated
				{
					*down_time_s = game_time + button_component.wait;
				}
				else if game_time >= *down_time_s
				{
					button_component.state = ButtonState::TargetReleased;
				}
			},
		}
	}
}

pub fn update_trains(ecs: &mut hecs::World, game_time: f32, time_delta_s: f32)
{
	for (id, (train_component, activation_component)) in ecs
		.query::<(&mut TrainComponent, &mut EntityActivationComponent)>()
		.iter()
	{
		loop
		{
			match &mut train_component.state
			{
				TrainState::SearchForInitialPosition =>
				{
					let mut q = ecs
						.query_one::<(&TargetNameComponent,)>(train_component.target)
						.unwrap();
					let target_name = &q.get().unwrap().0.name;

					for (target_id, (named_target_component,)) in ecs.query::<(&NamedTargetComponent,)>().iter()
					{
						if named_target_component.name == *target_name
						{
							// Just started. Set location to location of first target.
							let mut dst_q = ecs.query_one::<(&LocationComponent,)>(target_id).unwrap();
							let dst_position = dst_q.get().unwrap().0.position;
							drop(dst_q);

							let mut q = ecs
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
					let mut q = ecs
						.query_one::<(&TargetNameComponent,)>(train_component.target)
						.unwrap();
					let target_name = &q.get().unwrap().0.name;

					for (target_id, (named_target_component,)) in ecs.query::<(&NamedTargetComponent,)>().iter()
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
					let mut dst_q = ecs
						.query_one::<(&LocationComponent, &WaitComponent)>(train_component.target)
						.unwrap();
					let (dst_location_component, dst_wait_component) = dst_q.get().unwrap();
					let dst_position = dst_location_component.position + train_component.target_shift;
					let dst_wait = dst_wait_component.wait;
					drop(dst_q);

					let mut q = ecs.query_one::<(&mut LocationComponent,)>(id).unwrap();
					let position = &mut q.get().unwrap().0.position;

					*position = move_towards_target(position, &dst_position, train_component.speed, time_delta_s);

					if *position == dst_position
					{
						if dst_wait < 0.0
						{
							// Wait forever.
							train_component.state = TrainState::Wait {
								continue_time_s: game_time + 1.0e12,
							};
						}
						else if dst_wait > 0.0
						{
							train_component.state = TrainState::Wait {
								continue_time_s: game_time + dst_wait,
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
					if game_time >= *continue_time_s
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

pub fn update_kinematic_physics_objects(ecs: &mut hecs::World, physics: &mut TestGamePhysics)
{
	for (_id, (location_kinematic_physics_object_component, location_component)) in
		ecs.query_mut::<(&LocationKinematicPhysicsObjectComponent, &LocationComponent)>()
	{
		physics.set_kinematic_object_position(
			location_kinematic_physics_object_component.phys_handle,
			&location_component.position,
		);
	}
}

pub fn update_touch_triggers(ecs: &mut hecs::World, physics: &TestGamePhysics)
{
	for (_id, (touch_trigger_component, trigger_single_target_component, target_name_component)) in ecs
		.query::<(
			&TouchTriggerComponent,
			Option<&TriggerSingleTargetComponent>,
			Option<&TargetNameComponent>,
		)>()
		.iter()
	{
		physics.get_box_touching_entities(&touch_trigger_component.bbox, |entity| {
			// Only player can activate triggers.
			if let Ok(mut q) = ecs.query_one::<(&mut PlayerComponent,)>(entity)
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
				if let Ok(mut q) = ecs.query_one::<(&mut EntityActivationComponent,)>(t.target)
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
				for (target_id, (named_target_component,)) in ecs.query::<(&NamedTargetComponent,)>().iter()
				{
					if named_target_component.name == *name
					{
						if let Ok(mut q) = ecs.query_one::<(&mut EntityActivationComponent,)>(target_id)
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

pub fn update_named_activations(ecs: &mut hecs::World)
{
	for (id, (target_name_component,)) in ecs.query::<(&TargetNameComponent,)>().iter()
	{
		let mut activated = false;
		if let Ok(mut q) = ecs.query_one::<(&EntityActivationComponent,)>(id)
		{
			if let Some((actication_component,)) = q.get()
			{
				activated = actication_component.activated;
			}
		}

		if activated
		{
			for (target_id, (named_target_component,)) in ecs.query::<(&NamedTargetComponent,)>().iter()
			{
				if named_target_component.name == target_name_component.name
				{
					if let Ok(mut q) = ecs.query_one::<(&mut EntityActivationComponent,)>(target_id)
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

pub fn update_animations(ecs: &mut hecs::World, game_time: f32)
{
	for (_id, (_simple_animation_component, model)) in ecs.query_mut::<(&SimpleAnimationComponent, &mut ModelEntity)>()
	{
		let num_frames = model.model.frames_info.len() as u32;
		let frame_f = game_time * 10.0;
		model.animation.frames[0] = (frame_f as u32) % num_frames;
		model.animation.frames[1] = (frame_f as u32 + 1) % num_frames;
		model.animation.lerp = 1.0 - frame_f.fract();
	}
}

pub fn update_player_controller_locations(ecs: &mut hecs::World, physics: &TestGamePhysics)
{
	for (_id, (_player_controller_location_component, player_controller, location)) in ecs.query_mut::<(
		&PlayerControllerLocationComponent,
		&PlayerControllerComponent,
		&mut LocationComponent,
	)>()
	{
		location.position = match player_controller.position_source
		{
			PlayerPositionSource::Noclip(p) => p,
			PlayerPositionSource::Phys(handle) => physics.get_object_location(handle).0,
		};

		// Use only Z angle (do not rotate whole player up and down).
		location.rotation = QuaternionF::from_angle_z(
			player_controller.rotation_controller.get_azimuth() + Rad(std::f32::consts::PI * 0.5),
		);
	}
}

pub fn update_phys_model_locations(ecs: &mut hecs::World, physics: &TestGamePhysics)
{
	for (_id, (phys_handle, location)) in ecs.query_mut::<(&PhysicsLocationComponent, &mut LocationComponent)>()
	{
		let phys_location = physics.get_object_location(*phys_handle);
		location.position = phys_location.0;
		location.rotation = phys_location.1;
	}
}

pub fn update_other_entity_locations(ecs: &mut hecs::World)
{
	for (_id, (other_entity_location_component, location_component)) in ecs
		.query::<(&OtherEntityLocationComponent, &mut LocationComponent)>()
		.into_iter()
	{
		// TODO - support chains of linked entities.
		let mut q = ecs
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

pub fn update_player_controller_camera_locations(ecs: &mut hecs::World, physics: &TestGamePhysics)
{
	for (_id, (player_controller_camera_location_component, location_component)) in ecs
		.query::<(&PlayerControllerCameraLocationComponent, &mut LocationComponent)>()
		.into_iter()
	{
		let mut q = ecs
			.query_one::<(&PlayerControllerComponent,)>(player_controller_camera_location_component.entity)
			.unwrap();
		let (player_controller,) = q.get().unwrap();

		let camera_position = match player_controller.position_source
		{
			PlayerPositionSource::Noclip(p) => p,
			PlayerPositionSource::Phys(handle) => physics.get_object_location(handle).0,
		} + player_controller_camera_location_component.camera_view_offset;

		let camera_rotation = player_controller.rotation_controller.get_rotation();

		location_component.position = camera_position +
			camera_rotation.rotate_vector(player_controller_camera_location_component.relative_position);
		location_component.rotation = player_controller.rotation_controller.get_rotation() *
			player_controller_camera_location_component.relative_rotation;
	}
}

pub fn update_models_locations(ecs: &mut hecs::World)
{
	for (_id, (_model_entity_location_link_component, location_component, model)) in
		ecs.query_mut::<(&ModelEntityLocationLinkComponent, &LocationComponent, &mut ModelEntity)>()
	{
		model.position = location_component.position;
		model.rotation = location_component.rotation;
	}
}

pub fn update_submodels_locations(ecs: &mut hecs::World)
{
	for (_id, (_submodel_entity_with_index_location_component, location_component, submodel_entity_with_index)) in ecs
		.query_mut::<(
			&SubmodelEntityWithIndexLocationLinkComponent,
			&LocationComponent,
			&mut SubmodelEntityWithIndex,
		)>()
	{
		submodel_entity_with_index.submodel_entity.position = location_component.position;
		submodel_entity_with_index.submodel_entity.rotation = location_component.rotation;
	}
}

pub fn update_decals_locations(ecs: &mut hecs::World)
{
	for (_id, (_decal_location_component, location_component, decal)) in
		ecs.query_mut::<(&DecalLocationLinkComponent, &LocationComponent, &mut Decal)>()
	{
		decal.position = location_component.position;
		decal.rotation = location_component.rotation;
	}
}

pub fn update_sprites_locations(ecs: &mut hecs::World)
{
	for (_id, (_sprite_location_component, location_component, sprite)) in
		ecs.query_mut::<(&SpriteLocationLinkComponent, &LocationComponent, &mut Sprite)>()
	{
		sprite.position = location_component.position;
	}
}

pub fn update_dynamic_lights_locations(ecs: &mut hecs::World)
{
	for (_id, (_decal_location_component, location_component, dynamic_light)) in ecs.query_mut::<(
		&DynamicLightLocationLinkComponent,
		&LocationComponent,
		&mut DynamicLight,
	)>()
	{
		dynamic_light.position = location_component.position;
		if let DynamicLightShadowType::Projector { rotation, .. } = &mut dynamic_light.shadow_type
		{
			*rotation = location_component.rotation;
		}
	}
}

fn move_towards_target(position: &Vec3f, target_position: &Vec3f, speed: f32, time_delta: f32) -> Vec3f
{
	let step = speed * time_delta;
	debug_assert!(step >= 0.0);

	let vec_to = target_position - position;
	let vec_to_len = vec_to.magnitude();
	if vec_to_len <= 0.0 || vec_to_len <= step
	{
		return *target_position;
	}

	position + vec_to * (step / vec_to_len)
}
