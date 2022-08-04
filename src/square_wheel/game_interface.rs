use super::frame_info::*;
use common::system_window;

use super::{commands_processor::*, console::*, resources_manager::*};
use common::bsp_map_compact::*;
use std::sync::Arc;

pub trait GameInterface: Send + Sync
{
	fn process_input(&mut self, keyboard_state: &system_window::KeyboardState, time_delta_s: f32);
	fn update(&mut self, time_delta_s: f32);

	fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> FrameInfo;
}

pub type GameCreationFunction =
	fn(CommandsProcessorPtr, ConsoleSharedPtr, ResourcesManagerSharedPtr, Arc<BSPMap>) -> Box<dyn GameInterface>;
