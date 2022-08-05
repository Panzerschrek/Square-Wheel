use super::{commands_processor::*, console::*, frame_info::*, resources_manager::*};
use crate::common::{bsp_map_compact::*, color::*, system_window};
use std::sync::Arc;

pub trait GameInterface: Send + Sync
{
	fn update(&mut self, keyboard_state: &system_window::KeyboardState, time_delta_s: f32);

	fn get_frame_info(&self, surface_info: &system_window::SurfaceInfo) -> FrameInfo;

	fn draw_frame_overlay(&self, _pixels: &mut [Color32], _surface_info: &system_window::SurfaceInfo) {}
}

pub type GameInterfacePtr = Box<dyn GameInterface>;

pub type GameCreationFunction =
	fn(CommandsProcessorPtr, ConsoleSharedPtr, ResourcesManagerSharedPtr, Arc<BSPMap>) -> GameInterfacePtr;
