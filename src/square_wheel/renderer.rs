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
	map: Arc<bsp_map_compact::BSPMap>,
	root_renderer: PartialRenderer,
	debug_stats: RendererDebugStats,
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
			map: map.clone(),
			root_renderer: PartialRenderer::new(resources_manager, config_parsed, map, depth),
			debug_stats: RendererDebugStats::default(),
		}
	}

	pub fn prepare_frame<ColorT: AbstractColor>(
		&mut self,
		surface_info: &system_window::SurfaceInfo,
		frame_info: &FrameInfo,
	)
	{
		self.synchronize_config();

		self.debug_stats = RendererDebugStats::default();

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
			&mut self.debug_stats,
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
			&mut self.debug_stats,
		);

		self.print_debug_stats(debug_stats_printer);
	}

	fn print_debug_stats(&self, debug_stats_printer: &mut DebugStatsPrinter)
	{
		let performance_counters_ptr = self.root_renderer.get_performance_counters();
		let performance_counters = performance_counters_ptr.lock().unwrap();

		debug_stats_printer.add_line(format!(
			"materials update: {:04.2}ms",
			performance_counters.materials_update.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"visible leafs search: {:04.2}ms",
			performance_counters.visible_leafs_search.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"triangle models preparation: {:04.2}ms",
			performance_counters.triangle_models_preparation.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"surfaces preparation: {:04.2}ms",
			performance_counters.surfaces_preparation.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"shadow maps building: {:04.2}ms",
			performance_counters.shadow_maps_building.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"background fill: {:04.2}ms",
			performance_counters.background_fill.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"portals rendering: {:04.2}ms",
			performance_counters.portals_rendering.get_average_value() * 1000.0
		));
		debug_stats_printer.add_line(format!(
			"rasterization: {:04.2}ms",
			performance_counters.rasterization.get_average_value() * 1000.0
		));

		let debug_stats = &self.debug_stats;

		debug_stats_printer.add_line(format!(
			"leafs: {}/{}",
			debug_stats.num_visible_leafs,
			self.map.leafs.len()
		));
		debug_stats_printer.add_line(format!("submodels parts: {}", debug_stats.num_visible_submodels_parts));
		debug_stats_printer.add_line(format!("polygons: {}", debug_stats.num_visible_polygons));
		debug_stats_printer.add_line(format!(
			"dynamic meshes : {}, parts: {}, triangles: {}, vertices: {}",
			debug_stats.num_visible_meshes,
			debug_stats.num_visible_meshes_parts,
			debug_stats.num_triangles,
			debug_stats.num_triangle_vertices
		));
		debug_stats_printer.add_line(format!(
			"decals: {}, (parts in leafs: {})",
			debug_stats.num_decals, debug_stats.num_decals_leafs_parts
		));
		debug_stats_printer.add_line(format!(
			"sprites: {}, (parts in leafs: {})",
			debug_stats.num_sprites, debug_stats.num_sprites_leafs_parts
		));
		debug_stats_printer.add_line(format!(
			"dynamic lights: {}, (with shadow: {})",
			debug_stats.num_visible_lights, debug_stats.num_visible_lights_with_shadow
		));
		debug_stats_printer.add_line(format!(
			"visible portals: {}, pixels: {}k",
			debug_stats.num_visible_portals,
			(debug_stats.num_portals_pixels + 1023) / 1024
		));
		debug_stats_printer.add_line(format!(
			"surfaces pixels: {}k",
			(debug_stats.num_surfaces_pixels + 1023) / 1024
		));
		// debug_stats_printer.add_line(format!("mip bias: {}", self.mip_bias));
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
