use super::{commands_processor, commands_queue, console, frame_info::*, light::*, resources_manager::*};
use common::{camera_controller, math_types::*, matrix::*, system_window};

pub struct Game
{
	commands_processor: commands_processor::CommandsProcessorPtr,
	console: console::ConsoleSharedPtr,
	resources_manager: ResourcesManagerSharedPtr,
	commands_queue: commands_queue::CommandsQueuePtr<Game>,
	camera: camera_controller::CameraController,
	test_lights: Vec<PointLight>,
	test_models: Vec<ModelEntity>,
	game_time: f32,
}

impl Game
{
	pub fn new(
		commands_processor: commands_processor::CommandsProcessorPtr,
		console: console::ConsoleSharedPtr,
		resources_manager: ResourcesManagerSharedPtr,
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
		]);

		commands_processor
			.borrow_mut()
			.register_command_queue(commands_queue.clone() as commands_queue::CommandsQueueDynPtr);

		Self {
			commands_processor,
			console,
			resources_manager,
			commands_queue,
			camera: camera_controller::CameraController::new(),
			test_lights: Vec::new(),
			test_models: Vec::new(),
			game_time: 0.0,
		}
	}

	pub fn process_input(&mut self, keyboard_state: &sdl2::keyboard::KeyboardState, time_delta_s: f32)
	{
		self.camera.update(keyboard_state, time_delta_s);
	}

	pub fn update(&mut self, time_delta_s: f32)
	{
		self.process_commands();
		self.game_time += time_delta_s;

		for model in &mut self.test_models
		{
			model.frame = (self.game_time * 10.0) as u32 % (model.model.frames_info.len() as u32);
		}
	}

	pub fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> FrameInfo
	{
		let fov = std::f32::consts::PI * 0.375;
		let camera_matrices = build_view_matrix(
			self.camera.get_pos(),
			self.camera.get_azimuth(),
			self.camera.get_elevation(),
			fov,
			surface_info.width as f32,
			surface_info.height as f32,
		);

		FrameInfo {
			camera_matrices,
			game_time_s: self.game_time,
			lights: self.test_lights.clone(),
			model_entities: self.test_models.clone(),
		}
	}

	fn process_commands(&mut self)
	{
		let queue_ptr_copy = self.commands_queue.clone();
		queue_ptr_copy.borrow_mut().process_commands(self);
	}

	fn command_get_pos(&mut self, _args: commands_queue::CommandArgs)
	{
		let pos = self.camera.get_pos();
		self.console
			.borrow_mut()
			.add_text(format!("{} {} {}", pos.x, pos.y, pos.z));
	}

	fn command_set_pos(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console.borrow_mut().add_text("Expected 3 args".to_string());
			return;
		}

		if let (Ok(x), Ok(y), Ok(z)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			self.camera.set_pos(&Vec3f::new(x, y, z));
		}
		else
		{
			self.console.borrow_mut().add_text("Failed to parse args".to_string());
		}
	}

	fn command_get_angles(&mut self, _args: commands_queue::CommandArgs)
	{
		let angles = self.camera.get_angles();
		self.console.borrow_mut().add_text(format!("{} {}", angles.0, angles.1));
	}

	fn command_set_angles(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 2
		{
			self.console.borrow_mut().add_text("Expected 2 args".to_string());
			return;
		}

		if let (Ok(azimuth), Ok(elevation)) = (args[0].parse::<f32>(), args[1].parse::<f32>())
		{
			self.camera.set_angles(azimuth, elevation);
		}
		else
		{
			self.console.borrow_mut().add_text("Failed to parse args".to_string());
		}
	}

	fn command_add_test_light(&mut self, args: commands_queue::CommandArgs)
	{
		if args.len() < 3
		{
			self.console.borrow_mut().add_text("Expected 3 args".to_string());
			return;
		}

		if let (Ok(r), Ok(g), Ok(b)) = (args[0].parse::<f32>(), args[1].parse::<f32>(), args[2].parse::<f32>())
		{
			self.test_lights.push(PointLight {
				pos: self.camera.get_pos(),
				color: [r * 1024.0, g * 1024.0, b * 1024.0],
			});
		}
		else
		{
			self.console.borrow_mut().add_text("Failed to parse args".to_string());
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
			self.console.borrow_mut().add_text("Expected 2 args".to_string());
			return;
		}

		let model = self.resources_manager.lock().unwrap().get_model(&args[0]);
		let texture = self.resources_manager.lock().unwrap().get_image(&args[1]);

		self.test_models.push(ModelEntity {
			position: self.camera.get_pos(),
			angle_z: self.camera.get_azimuth(),
			frame: 0,
			model,
			texture,
			ordering_custom_bbox: None,
		});
	}

	fn command_reset_test_models(&mut self, _args: commands_queue::CommandArgs)
	{
		self.test_models.clear();
	}
}

impl Drop for Game
{
	fn drop(&mut self)
	{
		let commands_processor = self.commands_processor.clone();
		commands_processor
			.borrow_mut()
			.remove_command_queue(&(self.commands_queue.clone() as commands_queue::CommandsQueueDynPtr));
	}
}
