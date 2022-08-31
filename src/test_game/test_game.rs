use super::{
	commands_processor, commands_queue, components::*, config, console, frame_info::*, game_interface::*,
	resources_manager::*, test_game_physics, world_spawn, world_update,
};
use square_wheel_lib::common::{bsp_map_compact, color::*, material, math_types::*, matrix::*, system_window};
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
	camera_view_offset: Vec3f,
}

impl Game
{
	pub fn new(
		_config: config::ConfigSharedPtr,
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
			("add_test_projector_light", Game::command_add_test_projector_light),
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

		let mut ecs = hecs::World::new();
		let mut physics = test_game_physics::TestGamePhysics::new(map.clone());
		world_spawn::spawn_regular_entities(&mut ecs, &mut physics, &mut resources_manager.lock().unwrap(), &map);
		let player_entity =
			world_spawn::spawn_player(&mut ecs, &mut physics, &mut resources_manager.lock().unwrap(), &map);

		Self {
			commands_processor,
			console,
			resources_manager,
			commands_queue,
			commands_queue_dyn,
			map: map,
			physics,
			game_time: 0.0,
			ecs,
			ecs_command_buffer: hecs::CommandBuffer::new(),
			player_entity,
			camera_view_offset: Vec3f::new(0.0, 0.0, 22.0),
		}
	}

	fn collect_drawable_components<T: hecs::Component + Clone>(&self) -> Vec<T>
	{
		self.ecs.query::<(&T,)>().iter().map(|(_id, (c,))| c.clone()).collect()
	}

	fn get_camera_location(&self) -> (Vec3f, QuaternionF)
	{
		let mut q = self
			.ecs
			.query_one::<(&PlayerControllerComponent,)>(self.player_entity)
			.unwrap();
		let (player_controller_component,) = q.get().unwrap();

		(
			match player_controller_component.position_source
			{
				PlayerPositionSource::Noclip(p) => p,
				PlayerPositionSource::Phys(handle) => self.physics.get_object_location(handle).0,
			} + self.camera_view_offset,
			player_controller_component.rotation_controller.get_rotation(),
		)
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
			let pos = Vec3f::new(x, y, z) - self.camera_view_offset;
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
			self.console
				.lock()
				.unwrap()
				.add_text("Expected at least 3 args".to_string());
			return;
		}

		if let (Ok(r), Ok(g), Ok(b)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			self.ecs.spawn((
				TestLightComponent {},
				DynamicLight {
					position: self.get_camera_location().0,
					color: [r * 1024.0, g * 1024.0, b * 1024.0],
					radius: 512.0,
					shadow_type: if args.len() >= 4 && args[3] == "cube_shadow"
					{
						DynamicLightShadowType::Cubemap
					}
					else
					{
						DynamicLightShadowType::None
					},
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

	fn command_add_test_projector_light(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console
				.lock()
				.unwrap()
				.add_text("Expected at least 3 args".to_string());
			return;
		}

		if let (Ok(r), Ok(g), Ok(b)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			let (position, rotation) = self.get_camera_location();

			self.ecs.spawn((
				TestLightComponent {},
				DynamicLight {
					position,
					color: [r * 1024.0, g * 1024.0, b * 1024.0],
					radius: 2048.0,
					shadow_type: DynamicLightShadowType::Projector { rotation },
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
		let rotation = QuaternionF::one();

		let view_model_entity = self.ecs.spawn((
			SimpleAnimationComponent {},
			PlayerControllerCameraLocationComponent {
				entity: self.player_entity,
				camera_view_offset: self.camera_view_offset,
				relative_position: Vec3f::new(16.0, -8.0, -10.0),
				relative_rotation: QuaternionF::one(),
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
				blending_mode: material::BlendingMode::None,
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

				PlayerPositionSource::Phys(world_spawn::create_player_phys_object(
					&mut self.physics,
					self.player_entity,
					&pos,
				))
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

		world_update::update_player_entity(
			&mut self.ecs,
			&mut self.physics,
			self.player_entity,
			keyboard_state,
			events,
			self.game_time,
			time_delta_s,
		);
		world_update::despawn_timed_entites(&mut self.ecs, &mut self.ecs_command_buffer, self.game_time);
		world_update::update_test_projectiles(&mut self.ecs, time_delta_s);
		world_update::update_plates(&mut self.ecs, self.game_time, time_delta_s);
		world_update::update_doors(&mut self.ecs, self.game_time, time_delta_s);
		world_update::update_buttons(&mut self.ecs, self.game_time, time_delta_s);
		world_update::update_trains(&mut self.ecs, self.game_time, time_delta_s);
		world_update::update_kinematic_physics_objects(&mut self.ecs, &mut self.physics);

		// Update physics only after settig params of externally-controlled physics objects - player, platforms, etc.
		self.physics.update(time_delta_s);

		// Update models after physics update in order to setup position properly.

		world_update::update_touch_triggers(&mut self.ecs, &self.physics);
		world_update::update_named_activations(&mut self.ecs);

		world_update::update_animations(&mut self.ecs, self.game_time);

		// Update location of player entity, taken from player controller.
		world_update::update_player_controller_locations(&mut self.ecs, &self.physics);
		// Take locations from physics engine.
		world_update::update_phys_model_locations(&mut self.ecs, &self.physics);
		// Take locations from other entities. This is needed for entities, attached to other entities.
		world_update::update_other_entity_locations(&mut self.ecs);
		world_update::update_player_controller_camera_locations(&mut self.ecs, &self.physics);

		// Update locations of visible models.
		world_update::update_models_locations(&mut self.ecs);
		world_update::update_submodels_locations(&mut self.ecs);
		world_update::update_decals_locations(&mut self.ecs);
		world_update::update_dynamic_lights_locations(&mut self.ecs);
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
			skybox_rotation: QuaternionF::one(),
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
