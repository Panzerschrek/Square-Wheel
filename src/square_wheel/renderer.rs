use super::{
	abstract_color::*, config, debug_stats_printer::*, frame_info::*, partial_renderer::PartialRenderer,
	renderer_config::*, resources_manager::*,
};
use crate::common::{bsp_map_compact, system_window};
use std::sync::Arc;

pub struct Renderer
{
	app_config: config::ConfigSharedPtr,
	config: RendererConfig,
	root_renderer: PartialRenderer,
}

impl Renderer
{
	pub fn new(
		resources_manager: ResourcesManagerSharedPtr,
		app_config: config::ConfigSharedPtr,
		map: Arc<bsp_map_compact::BSPMap>,
	) -> Self
	{
		let config_parsed = RendererConfig::from_app_config(&app_config);
		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		let depth = 2; // TODO - read from config
		Self {
			app_config,
			config: config_parsed,
			root_renderer: PartialRenderer::new(resources_manager, config_parsed, map, depth),
		}
	}

	pub fn prepare_frame<ColorT: AbstractColor>(
		&mut self,
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
	)
	{
		self.synchronize_config();
		self.root_renderer
			.prepare_frame::<ColorT>(surface_info, frame_info, &frame_info.camera_matrices)
	}

	pub fn draw_frame<ColorT: AbstractColor>(
		&mut self,
		pixels: &mut [ColorT],
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
		debug_stats_printer: &mut DebugStatsPrinter,
	)
	{
		self.root_renderer.draw_frame(
			pixels,
			surface_info,
			frame_info,
			&frame_info.camera_matrices,
			debug_stats_printer,
		)
	}

	fn synchronize_config(&mut self)
	{
		self.config = RendererConfig::from_app_config(&self.app_config);

		// Make sure that config values are reasonable.
		let mut config_is_dirty = false;
		if self.config.textures_mip_bias < -1.0
		{
			self.config.textures_mip_bias = -1.0;
			config_is_dirty = true;
		}
		if self.config.textures_mip_bias > 2.0
		{
			self.config.textures_mip_bias = 2.0;
			config_is_dirty = true;
		}

		if self.config.shadows_quality < -1.0
		{
			self.config.shadows_quality = -1.0;
			config_is_dirty = true;
		}
		if self.config.shadows_quality > 1.0
		{
			self.config.shadows_quality = 1.0;
			config_is_dirty = true;
		}

		if config_is_dirty
		{
			self.config.update_app_config(&self.app_config);
		}

		self.root_renderer.set_config(self.config);
	}
}
