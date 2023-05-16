use super::{
	abstract_color::*, config, debug_stats_printer::*, dynamic_objects_index::*, frame_info::*, inline_models_index::*,
	map_materials_processor::*, partial_renderer::PartialRenderer, renderer_config::*, renderer_structs::*,
	resources_manager::*,
};
use crate::common::{bsp_map_compact, system_window};
use std::sync::Arc;

pub struct Renderer
{
	app_config: config::ConfigSharedPtr,
	config: RendererConfig,
	common_data: RenderersCommonData,
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
			common_data: RenderersCommonData {
				materials_processor: MapMaterialsProcessor::new(resources_manager.clone(), &*map),
				inline_models_index: InlineModelsIndex::new(map.clone()),
				dynamic_models_index: DynamicObjectsIndex::new(map.clone()),
				decals_index: DynamicObjectsIndex::new(map.clone()),
				sprites_index: DynamicObjectsIndex::new(map.clone()),
				dynamic_lights_index: DynamicObjectsIndex::new(map.clone()),
				portals_index: DynamicObjectsIndex::new(map.clone()),
				leafs_planes: (0 .. map.leafs.len())
					.map(|leaf_index| get_leaf_clip_planes(&map, leaf_index as u32))
					.collect(),
			},
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

		self.common_data.materials_processor.update(frame_info.game_time_s);

		self.common_data
			.inline_models_index
			.position_models(&frame_info.submodel_entities);
		self.common_data
			.dynamic_models_index
			.position_models(&frame_info.model_entities);
		self.common_data.decals_index.position_decals(&frame_info.decals);
		self.common_data.sprites_index.position_sprites(&frame_info.sprites);
		self.common_data
			.dynamic_lights_index
			.position_dynamic_lights(&frame_info.lights);
		self.common_data.portals_index.position_portals(&frame_info.portals);

		self.root_renderer.prepare_frame::<ColorT>(
			surface_info,
			frame_info,
			&frame_info.camera_matrices,
			&self.common_data,
		)
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
			&self.common_data,
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
