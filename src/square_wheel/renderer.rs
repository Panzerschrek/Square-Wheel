use super::{
	abstract_color::*, config, console::*, debug_stats_printer::*, dynamic_objects_index::*, frame_info::*,
	inline_models_index::*, map_materials_processor::*, partial_renderer::PartialRenderer, performance_counter::*,
	renderer_config::*, renderer_structs::*, resources_manager::*,
};
use crate::common::{bsp_map_compact, system_window};
use std::sync::Arc;

pub struct Renderer
{
	app_config: config::ConfigSharedPtr,
	config: RendererConfig,
	console: ConsoleSharedPtr,
	common_data: RenderersCommonData,
	map: Arc<bsp_map_compact::BSPMap>,
	root_renderer: PartialRenderer,
	materials_update_performance_counter: PerformanceCounter,
	object_index_build_performance_counter: PerformanceCounter,
	debug_stats: RendererDebugStats,
}

impl Renderer
{
	pub fn new(
		resources_manager: ResourcesManagerSharedPtr,
		app_config: config::ConfigSharedPtr,
		console: ConsoleSharedPtr,
		map: Arc<bsp_map_compact::BSPMap>,
	) -> Self
	{
		let mut config_parsed = RendererConfig::from_app_config(&app_config);

		config_parsed.portals_depth = std::cmp::min(config_parsed.portals_depth, 8);

		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		Self {
			app_config: app_config.clone(),
			config: config_parsed,
			console,
			common_data: RenderersCommonData {
				materials_processor: MapMaterialsProcessor::new(resources_manager.clone(), app_config, &*map),
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
			root_renderer: PartialRenderer::new(resources_manager, config_parsed, map, config_parsed.portals_depth),
			materials_update_performance_counter: PerformanceCounter::new(100),
			object_index_build_performance_counter: PerformanceCounter::new(100),
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

		let common_data = &mut self.common_data;
		let materials_update_performance_counter = &mut self.materials_update_performance_counter;
		let object_index_build_performance_counter = &mut self.object_index_build_performance_counter;

		let frame_world_info = &frame_info.world;

		materials_update_performance_counter
			.run_with_measure(|| common_data.materials_processor.update(frame_world_info.game_time_s));

		object_index_build_performance_counter.run_with_measure(|| {
			common_data
				.inline_models_index
				.position_models(&frame_world_info.submodel_entities);
			common_data
				.dynamic_models_index
				.position_models(&frame_world_info.model_entities);
			common_data.decals_index.position_decals(&frame_world_info.decals);
			common_data.sprites_index.position_sprites(&frame_world_info.sprites);
			common_data
				.dynamic_lights_index
				.position_dynamic_lights(&frame_world_info.lights);
			common_data.portals_index.position_portals(&frame_world_info.portals);
		});

		self.root_renderer.prepare_frame::<ColorT>(
			surface_info,
			frame_world_info,
			&frame_info.view.camera_matrices,
			frame_info.view.is_third_person_view,
			None,
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
			&frame_info.world,
			&frame_info.view.camera_matrices,
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
			"materials update: {:04.2}ms, animated texels: {}k",
			self.materials_update_performance_counter.get_average_value() * 1000.0,
			(self.common_data.materials_processor.get_num_animated_texels() + 1023) / 1024
		));
		debug_stats_printer.add_line(format!(
			"object index build: {:04.2}ms",
			self.object_index_build_performance_counter.get_average_value() * 1000.0
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
		let config_updated = RendererConfig::from_app_config(&self.app_config);
		if config_updated.portals_depth != self.config.portals_depth
		{
			self.console
				.lock()
				.unwrap()
				.add_text("portals_depth setting will be applied after map reloading".to_string());
		}

		self.config = config_updated;

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
