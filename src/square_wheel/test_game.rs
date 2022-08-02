use super::{
	commands_processor, commands_queue, console, frame_info::*, light::*, resources_manager::*, test_game_physics,
};
use common::{
	bsp_map_compact, camera_controller::*, camera_rotation_controller::*, material, math_types::*, matrix::*,
	system_window,
};
use std::sync::Arc;

pub struct Game
{
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	resources_manager: ResourcesManagerSharedPtr,
	commands_queue: commands_queue::CommandsQueuePtr<Game>,
	physics: test_game_physics::TestGamePhysics,
	player_controller: PlayerController,
	submodels: Vec<SubmodelEntityOpt>,
	test_lights: Vec<PointLight>,
	test_models: Vec<PhysicsTestModel>,
	view_model: Option<ModelEntity>,
	game_time: f32,
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
			("set_view_model", Game::command_set_view_model),
			("reset_view_model", Game::command_reset_view_model),
			("noclip", Game::command_noclip),
		]);

		commands_processor
			.lock()
			.unwrap()
			.register_command_queue(commands_queue.clone() as commands_queue::CommandsQueueDynPtr);

		let submodels = vec![None; map.submodels.len()];

		Self {
			commands_processor,
			console,
			resources_manager,
			commands_queue,
			player_controller: PlayerController::NoclipController(CameraController::new()),
			physics: test_game_physics::TestGamePhysics::new(map),
			submodels,
			test_lights: Vec::new(),
			test_models: Vec::new(),
			view_model: None,
			game_time: 0.0,
		}
	}

	pub fn process_input(&mut self, keyboard_state: &system_window::KeyboardState, time_delta_s: f32)
	{
		match &mut self.player_controller
		{
			PlayerController::NoclipController(camera_controller) =>
			{
				camera_controller.update(keyboard_state, time_delta_s)
			},
			PlayerController::PhysicsController(physics_controller) =>
			{
				physics_controller
					.rotation_controller
					.update(keyboard_state, time_delta_s);

				let azimuth = physics_controller.rotation_controller.get_angles().0;
				let forward_vector = Vec3f::new(-(azimuth.sin()), azimuth.cos(), 0.0);
				let left_vector = Vec3f::new(azimuth.cos(), azimuth.sin(), 0.0);
				let mut move_vector = Vec3f::new(0.0, 0.0, 0.0);
				let acceleration = 1536.0;

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
					move_vector = move_vector * (time_delta_s * acceleration / move_vector_length);
				}

				// TODO - limit maximum velocity.

				let on_ground = self.physics.is_object_on_ground(physics_controller.phys_handle);
				let cur_velocity = self.physics.get_object_velocity(physics_controller.phys_handle);
				if keyboard_state.contains(&Scancode::Space) && on_ground && cur_velocity.z <= 1.0
				{
					move_vector.z = 384.0;
				}

				self.physics
					.add_object_velocity(physics_controller.phys_handle, &move_vector);
			},
		}
	}

	pub fn update(&mut self, time_delta_s: f32)
	{
		self.process_commands();

		self.game_time += time_delta_s;
		self.physics.update(time_delta_s);

		let test_animate_submodels = false;
		if test_animate_submodels
		{
			for (index, submodel_opt) in self.submodels.iter_mut().enumerate()
			{
				let phase = index as f32;
				*submodel_opt = Some(SubmodelEntity {
					angle_z: Rad((0.0625 * self.game_time + phase).sin() * 0.25),
					shift: 32.0 *
						Vec3f::new(
							(0.5 * self.game_time + phase).sin(),
							(0.33 * self.game_time + phase).sin(),
							(0.11111 * self.game_time + phase).sin(),
						),
				});
			}

			if !self.submodels.is_empty()
			{
				let hide_submodel_index = self.game_time as usize % self.submodels.len();
				self.submodels[hide_submodel_index] = None;
			}
		}

		for model in &mut self.test_models
		{
			let num_frames = model.draw_entity.model.frames_info.len() as u32;
			let frame_f = self.game_time * 10.0;
			model.draw_entity.animation.frames[0] = (frame_f as u32) % num_frames;
			model.draw_entity.animation.frames[1] = (frame_f as u32 + 1) % num_frames;
			model.draw_entity.animation.lerp = 1.0 - frame_f.fract();

			let location = self.physics.get_object_location(model.phys_handle);

			model.draw_entity.position = location.0;
			model.draw_entity.angles = location.1;
		}
	}

	pub fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> FrameInfo
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

		let mut model_entities = self
			.test_models
			.iter()
			.map(|e| e.draw_entity.clone())
			.collect::<Vec<_>>();
		if let Some(mut view_model) = self.view_model.clone()
		{
			let azimuth = self.get_camera_angles().0;

			// TODO - use also camera elevation.
			let shift_vec_front = Vec3f::new(-azimuth.sin(), azimuth.cos(), 0.0) * 16.0;
			let shift_vec_left = Vec3f::new(azimuth.cos(), azimuth.sin(), 0.0) * 8.0;
			let shift_vec_down = Vec3f::new(0.0, 0.0, -1.0) * 10.0;

			let (pos, angles) = self.get_camera_location();
			view_model.position = pos + shift_vec_front + shift_vec_left + shift_vec_down;
			view_model.angles = angles;
			model_entities.push(view_model);
		}

		FrameInfo {
			camera_matrices,
			submodel_entities: self.submodels.clone(),
			skybox_angles: EulerAnglesF::new(Rad(0.0), Rad(0.0), Rad(0.0)),
			game_time_s: self.game_time,
			lights: self.test_lights.clone(),
			model_entities,
		}
	}

	fn get_camera_location(&self) -> (Vec3f, EulerAnglesF)
	{
		match &self.player_controller
		{
			PlayerController::NoclipController(camera_controller) =>
			{
				(camera_controller.get_pos(), camera_controller.get_euler_angles())
			},
			PlayerController::PhysicsController(physics_controller) => (
				self.physics.get_object_location(physics_controller.phys_handle).0,
				physics_controller.rotation_controller.get_euler_angles(),
			),
		}
	}

	fn get_camera_angles(&self) -> (f32, f32, f32)
	{
		match &self.player_controller
		{
			PlayerController::NoclipController(camera_controller) => camera_controller.get_angles(),
			PlayerController::PhysicsController(physics_controller) =>
			{
				physics_controller.rotation_controller.get_angles()
			},
		}
	}

	fn set_camera_angles(&mut self, azimuth: f32, elevation: f32, roll: f32)
	{
		match &mut self.player_controller
		{
			PlayerController::NoclipController(camera_controller) =>
			{
				camera_controller.set_angles(azimuth, elevation, roll)
			},
			PlayerController::PhysicsController(physics_controller) => physics_controller
				.rotation_controller
				.set_angles(azimuth, elevation, roll),
		}
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
			match &mut self.player_controller
			{
				PlayerController::NoclipController(camera_controller) =>
				{
					camera_controller.set_pos(&pos);
				},
				PlayerController::PhysicsController(physics_controller) =>
				{
					// TODO
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
			self.test_lights.push(PointLight {
				pos: self.get_camera_location().0,
				color: [r * 1024.0, g * 1024.0, b * 1024.0],
			});
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
		self.test_lights.clear();
	}

	fn command_add_test_model(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.lock().unwrap().add_text("Expected 2 args".to_string());
			return;
		}

		let model = self.resources_manager.lock().unwrap().get_model(&args[0]);
		let texture = self.resources_manager.lock().unwrap().get_image(&args[1]);

		let (pos, angles) = self.get_camera_location();
		let bbox = model.frames_info[0].bbox;

		self.test_models.push(PhysicsTestModel {
			phys_handle: self.physics.add_object(&pos, &angles, &bbox),
			draw_entity: ModelEntity {
				position: pos,
				angles: angles,
				animation: AnimationPoint {
					frames: [0, 0],
					lerp: 0.0,
				},
				model,
				texture,
				blending_mode: material::BlendingMode::None,
				is_view_model: false,
				ordering_custom_bbox: None,
			},
		});
	}

	fn command_reset_test_models(&mut self, _args: commands_queue::CommandArgs)
	{
		for model in &self.test_models
		{
			self.physics.remove_object(model.phys_handle);
		}
		self.test_models.clear();
	}

	fn command_set_view_model(&mut self, args: commands_queue::CommandArgs)
	{
		self.view_model = None;
		if args.len() < 2
		{
			self.console.lock().unwrap().add_text("Expected 2 args".to_string());
			return;
		}

		let model = self.resources_manager.lock().unwrap().get_model(&args[0]);
		let texture = self.resources_manager.lock().unwrap().get_image(&args[1]);

		self.view_model = Some(ModelEntity {
			position: Vec3f::zero(),
			angles: EulerAnglesF::new(Rad(0.0), Rad(0.0), Rad(0.0)),
			animation: AnimationPoint {
				frames: [0, 0],
				lerp: 0.0,
			},
			model,
			texture,
			blending_mode: material::BlendingMode::Average,
			is_view_model: true,
			ordering_custom_bbox: None,
		});
	}

	fn command_reset_view_model(&mut self, _args: commands_queue::CommandArgs)
	{
		self.view_model = None;
	}

	fn command_noclip(&mut self, _args: commands_queue::CommandArgs)
	{
		let (pos, _) = self.get_camera_location();
		let angles = self.get_camera_angles();

		if let PlayerController::PhysicsController(physics_controller) = &self.player_controller
		{
			self.physics.remove_object(physics_controller.phys_handle);

			let mut camera_controller = CameraController::new();
			camera_controller.set_pos(&pos);
			camera_controller.set_angles(angles.0, angles.1, angles.2);

			self.player_controller = PlayerController::NoclipController(camera_controller);

			self.console.lock().unwrap().add_text("Noclip ON".to_string());
		}
		else
		{
			let mut rotation_controller = CameraRotationController::new();
			rotation_controller.set_angles(angles.0, angles.1, angles.2);

			let controller = PlayerPhysicsController {
				phys_handle: self.physics.add_character_object(&pos, 60.0, 120.0),
				rotation_controller,
			};

			self.player_controller = PlayerController::PhysicsController(controller);

			self.console.lock().unwrap().add_text("Noclip OFF".to_string());
		}
	}
}

impl Drop for Game
{
	fn drop(&mut self)
	{
		let commands_processor = self.commands_processor.clone();
		commands_processor
			.lock()
			.unwrap()
			.remove_command_queue(&(self.commands_queue.clone() as commands_queue::CommandsQueueDynPtr));
	}
}

struct PhysicsTestModel
{
	phys_handle: test_game_physics::ObjectHandle,
	draw_entity: ModelEntity,
}

enum PlayerController
{
	NoclipController(CameraController),
	PhysicsController(PlayerPhysicsController),
}

struct PlayerPhysicsController
{
	rotation_controller: CameraRotationController,
	phys_handle: test_game_physics::ObjectHandle,
}
