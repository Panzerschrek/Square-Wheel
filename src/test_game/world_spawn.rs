use super::{components::*, frame_info::*, resources_manager::*, test_game_physics::*};
use square_wheel_lib::common::{
	bbox::*, bsp_map_compact, camera_rotation_controller::*, map_file_common, material, math_types::*,
};

pub fn spawn_regular_entities(
	ecs: &mut hecs::World,
	physics: &mut TestGamePhysics,
	resources_manager: &mut ResourcesManager,
	map: &bsp_map_compact::BSPMap,
)
{
	// Skip world entity.
	for map_entity in &map.entities[1 ..]
	{
		spawn_regular_entity(ecs, physics, resources_manager, map, map_entity);
	}

	prepare_linked_doors(ecs, map);
}

fn spawn_regular_entity(
	ecs: &mut hecs::World,
	physics: &mut TestGamePhysics,
	resources_manager: &mut ResourcesManager,
	map: &bsp_map_compact::BSPMap,
	map_entity: &bsp_map_compact::Entity,
)
{
	let class_name = get_entity_classname(map_entity, map);
	match class_name
	{
		Some("trigger_multiple") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let entity = ecs.spawn((TouchTriggerComponent { bbox },));
				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("trigger_teleport") =>
		{
			// Use Quake teleports logic.
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let entity = ecs.spawn((TouchTriggerTeleportComponent {
					bbox: bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]),
				},));
				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("info_teleport_destination") =>
		{
			if let Some(origin) = get_entity_origin(map_entity, map)
			{
				let entity = ecs.spawn((LocationComponent {
					// Shift position a bit up - as Quake does.
					position: origin + Vec3f::new(0.0, 0.0, 27.0),
					rotation: get_entity_rotation(map_entity, map),
				},));
				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("path_corner") =>
		{
			if let Some(origin) = get_entity_origin(map_entity, map)
			{
				let entity = ecs.spawn((
					LocationComponent {
						position: origin,
						rotation: QuaternionF::one(),
					},
					WaitComponent {
						wait: get_entity_f32(map_entity, map, "wait").unwrap_or(0.0),
					},
				));

				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("func_plat") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let height = if let Some(h) = get_entity_f32(map_entity, map, "height")
				{
					h
				}
				else
				{
					bbox.max.z - bbox.min.z - 8.0
				};

				let position_upper = bbox.get_center();
				let position_lower = position_upper - Vec3f::new(0.0, 0.0, height);

				let position = position_lower;
				let rotation = QuaternionF::one();

				// TODO - start at top if plate requires activation by another entity.

				let entity = ecs.spawn(());
				ecs.insert(
					entity,
					(
						SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity { position, rotation },
						},
						LocationComponent { position, rotation },
						EntityActivationComponent { activated: false },
						PlateComponent {
							speed: get_entity_f32(map_entity, map, "speed").unwrap_or(150.0),
							position_lower,
							position_upper,
							state: PlateState::TargetDown,
						},
						// Update physics object using location component.
						LocationKinematicPhysicsObjectComponent {
							phys_handle: physics.add_submodel_object(
								entity,
								&bbox,
								&Vec3f::new(0.0, 0.0, -height),
								&rotation,
							),
						},
						// Update draw model location, using location component.
						SubmodelEntityWithIndexLocationLinkComponent {},
					),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);

				// Add activation trigger.
				// TODO - use PLAT_LOW_TRIGGER.
				let bbox_half_size = bbox.get_size() * 0.5;
				let bbox_reduce_min = Vec3f::new(
					(bbox_half_size.x - 1.0).min(25.0),
					(bbox_half_size.y - 1.0).min(25.0),
					0.0,
				);
				let bbox_reduce_max = Vec3f::new(
					(bbox_half_size.x - 1.0).min(25.0),
					(bbox_half_size.y - 1.0).min(25.0),
					-8.0,
				);

				let trigger_bbox = BBox::from_min_max(bbox.min + bbox_reduce_min, bbox.max - bbox_reduce_max);

				ecs.spawn((
					TouchTriggerComponent { bbox: trigger_bbox },
					TriggerSingleTargetComponent { target: entity },
				));
			}
		},
		Some("func_door") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let direction = get_entity_move_direction(map_entity, map);

				let lip = get_entity_f32(map_entity, map, "lip").unwrap_or(8.0);

				// TODO - use DOOR_START_OPEN.
				let position_closed = bbox.get_center();
				let position_opened = position_closed + direction * (direction.dot(bbox.get_size()).abs() - lip);

				let position = position_closed;
				let rotation = QuaternionF::one();

				let entity = ecs.spawn(());
				ecs.insert(
					entity,
					(
						SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity { position, rotation },
						},
						LocationComponent { position, rotation },
						EntityActivationComponent { activated: false },
						DoorComponent {
							speed: get_entity_f32(map_entity, map, "speed").unwrap_or(100.0),
							wait: get_entity_f32(map_entity, map, "wait").unwrap_or(3.0),
							position_opened,
							position_closed,
							state: DoorState::TargetClosed,
							slave_doors: Vec::new(), // set later
						},
						// Update physics object using location component.
						LocationKinematicPhysicsObjectComponent {
							phys_handle: physics.add_submodel_object(
								entity,
								&bbox,
								&(bbox.get_center() - position_closed),
								&rotation,
							),
						},
						// Update draw model location, using location component.
						SubmodelEntityWithIndexLocationLinkComponent {},
					),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);

				// Add trigger later - after linking touched doors.
			}
		},
		Some("func_button") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let direction = get_entity_move_direction(map_entity, map);

				let lip = get_entity_f32(map_entity, map, "lip").unwrap_or(4.0);

				let position_released = bbox.get_center();
				let position_pressed = position_released + direction * (direction.dot(bbox.get_size()).abs() - lip);

				let position = position_released;
				let rotation = QuaternionF::one();

				let entity = ecs.spawn(());
				ecs.insert(
					entity,
					(
						SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity { position, rotation },
						},
						LocationComponent { position, rotation },
						EntityActivationComponent { activated: false },
						ButtonComponent {
							speed: get_entity_f32(map_entity, map, "speed").unwrap_or(40.0),
							wait: get_entity_f32(map_entity, map, "wait").unwrap_or(1.0),
							position_released,
							position_pressed,
							state: ButtonState::TargetReleased,
						},
						// Update physics object using location component.
						LocationKinematicPhysicsObjectComponent {
							phys_handle: physics.add_submodel_object(
								entity,
								&bbox,
								&(bbox.get_center() - position_released),
								&rotation,
							),
						},
						// Update draw model location, using location component.
						SubmodelEntityWithIndexLocationLinkComponent {},
					),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);

				let bbox_increase = Vec3f::new(1.0, 1.0, 1.0);
				let trigger_bbox = BBox::from_min_max(bbox.min - bbox_increase, bbox.max + bbox_increase);

				ecs.spawn((
					TouchTriggerComponent { bbox: trigger_bbox },
					TriggerSingleTargetComponent { target: entity },
				));
			}
		},
		Some("func_train") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let position = bbox.get_center();
				let rotation = QuaternionF::one();

				let entity = ecs.spawn(());
				ecs.insert(
					entity,
					(
						SubmodelEntityWithIndex {
							index,
							submodel_entity: SubmodelEntity { position, rotation },
						},
						LocationComponent { position, rotation },
						EntityActivationComponent { activated: false },
						TrainComponent {
							speed: get_entity_f32(map_entity, map, "speed").unwrap_or(100.0),
							state: TrainState::SearchForInitialPosition,
							target: entity,
							// Shift target positions because in Quake position is regulated for minimum point of bbox.
							target_shift: bbox.get_size() * 0.5,
						},
						// Update physics object using location component.
						LocationKinematicPhysicsObjectComponent {
							phys_handle: physics.add_submodel_object(entity, &bbox, &Vec3f::zero(), &rotation),
						},
						// Update draw model location, using location component.
						SubmodelEntityWithIndexLocationLinkComponent {},
					),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("func_camera_portal") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				for polygon in bsp_map_compact::get_submodel_polygons(map, &map.submodels[index])
				{
					let entity = ecs.spawn((
						ViewPortal {
							view: PortalView::CameraAtPosition {
								position: Vec3f::zero(),
								rotation: QuaternionF::one(),
								fov: Rad(get_entity_f32(map_entity, map, "fov").unwrap_or(90.0) * TO_RAD),
							},
							plane: polygon.plane,
							tex_coord_equation: polygon.tex_coord_equation,
							vertices: Vec::from(bsp_map_compact::get_polygon_vertices(map, polygon)),
							blending_mode: get_entity_blending_mode(map_entity, map),
							texture: get_portal_texture(resources_manager, map, polygon.texture),
						},
						ViewPortalTargetLocationLinkComponent {},
					));

					add_entity_common_components(ecs, map, map_entity, entity);
				}
			}
		},
		// Name portal cameras like in Quake III.
		Some("misc_portal_camera") =>
		{
			if let Some(origin) = get_entity_origin(map_entity, map)
			{
				let entity = ecs.spawn((LocationComponent {
					position: origin,
					rotation: get_entity_rotation(map_entity, map),
				},));
				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("func_mirror") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				for polygon in bsp_map_compact::get_submodel_polygons(map, &map.submodels[index])
				{
					let mut tex_coord_equation = polygon.tex_coord_equation;
					if tex_coord_equation[0]
						.vec
						.cross(tex_coord_equation[1].vec)
						.dot(polygon.plane.vec) < 0.0
					{
						// Make sure mirror basis has proper orientation.
						tex_coord_equation[0] = tex_coord_equation[0].get_inverted();
					}

					let entity = ecs.spawn((ViewPortal {
						view: PortalView::Mirror {},
						plane: polygon.plane,
						tex_coord_equation,
						vertices: Vec::from(bsp_map_compact::get_polygon_vertices(map, polygon)),
						blending_mode: get_entity_blending_mode(map_entity, map),
						texture: get_portal_texture(resources_manager, map, polygon.texture),
					},));

					add_entity_common_components(ecs, map, map_entity, entity);
				}
			}
		},
		Some("func_parallax_portal") =>
		{
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				for polygon in bsp_map_compact::get_submodel_polygons(map, &map.submodels[index])
				{
					let mut tex_coord_equation = polygon.tex_coord_equation;
					if tex_coord_equation[0]
						.vec
						.cross(tex_coord_equation[1].vec)
						.dot(polygon.plane.vec) > 0.0
					{
						// Make sure portal basis has proper orientation.
						tex_coord_equation[0] = tex_coord_equation[0].get_inverted();
					}

					let entity = ecs.spawn((
						ViewPortal {
							view: PortalView::ParallaxPortal {
								transform_matrix: Mat4f::identity(),
							},
							plane: polygon.plane,
							tex_coord_equation,
							vertices: Vec::from(bsp_map_compact::get_polygon_vertices(map, polygon)),
							blending_mode: get_entity_blending_mode(map_entity, map),
							texture: get_portal_texture(resources_manager, map, polygon.texture),
						},
						ViewPortalTargetLocationLinkComponent {},
					));

					add_entity_common_components(ecs, map, map_entity, entity);
				}
			}
		},
		Some("misc_model") =>
		{
			if let (Some(model_file_name), Some(origin)) = (
				get_entity_key_value(map_entity, map, "model"),
				get_entity_origin(map_entity, map),
			)
			{
				let model = resources_manager.get_model(model_file_name);

				// TODO - load texture more wisely.
				let mut texture_file_name = String::new();
				for mesh in &model.meshes
				{
					if !mesh.material_name.is_empty()
					{
						texture_file_name = mesh.material_name.clone();
						break;
					}
				}

				let texture = resources_manager.get_texture_lite(&texture_file_name);

				let position = origin;
				let rotation = get_entity_rotation(map_entity, map);

				ecs.spawn((
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
						blending_mode: get_entity_blending_mode(map_entity, map),
						lighting: ModelLighting::Default,
						flags: ModelEntityDrawFlags::empty(),
						ordering_custom_bbox: None,
					},
				));
			}
		},
		Some("misc_sprite") =>
		{
			if let (Some(sprite_file_name), Some(origin)) = (
				get_entity_key_value(map_entity, map, "sprite"),
				get_entity_origin(map_entity, map),
			)
			{
				let scale = get_entity_f32(map_entity, map, "scale").unwrap_or(1.0);

				let texture = resources_manager.get_texture_lite(sprite_file_name);

				let texture_mip0 = &texture[0];
				let radius = scale *
					0.25 * ((texture_mip0.size[0] * texture_mip0.size[0] +
					texture_mip0.size[1] * texture_mip0.size[1]) as f32)
					.sqrt();

				// TODO - fix this mess, use string representations of properties, instead of meaningless numbers.

				let mut orientation = SpriteOrientation::FacingTowardsCamera;
				if let Some(entity_orientation) = get_entity_f32(map_entity, map, "orientation")
				{
					match entity_orientation as u32
					{
						0 =>
						{
							orientation = SpriteOrientation::ParallelToCameraPlane;
						},
						1 =>
						{
							orientation = SpriteOrientation::FacingTowardsCamera;
						},
						2 =>
						{
							orientation = SpriteOrientation::AlignToZAxisParallelToCameraPlane;
						},
						3 =>
						{
							orientation = SpriteOrientation::AlignToZAxisFacingTowardsCamera;
						},
						_ =>
						{},
					};
				}

				let mut light_add = [0.0, 0.0, 0.0];
				let mut light_scale = 1.0;
				if let Some(light) = get_entity_f32(map_entity, map, "light")
				{
					light_scale = 0.0;
					let mut color = [1.0, 1.0, 1.0];
					if let Some(entity_color_str) = get_entity_key_value(map_entity, map, "color")
					{
						if let Ok(entity_color) = map_file_common::parse_vec3(entity_color_str)
						{
							color[0] = (entity_color.x / 255.0).max(0.0).min(1.0);
							color[1] = (entity_color.y / 255.0).max(0.0).min(1.0);
							color[2] = (entity_color.z / 255.0).max(0.0).min(1.0);
						}
					}

					light_add = color.map(|c| c * light);
				}

				ecs.spawn((Sprite {
					position: origin,
					angle: 0.0,
					radius,
					texture,
					orientation,
					blending_mode: get_entity_blending_mode(map_entity, map),
					light_scale,
					light_add,
				},));
			}
		},
		Some("misc_decal") =>
		{
			if let (Some(decal_file_name), Some(origin)) = (
				get_entity_key_value(map_entity, map, "decal"),
				get_entity_origin(map_entity, map),
			)
			{
				let scale = get_entity_f32(map_entity, map, "scale").unwrap_or(1.0);

				let texture = resources_manager.get_texture_lite(decal_file_name);

				let texture_mip0 = &texture[0];
				let size = Vec3f::new(
					texture_mip0.size[0].min(texture_mip0.size[1]) as f32,
					texture_mip0.size[0] as f32,
					texture_mip0.size[1] as f32,
				) * 0.5 * scale;

				let mut light_add = [0.0, 0.0, 0.0];
				let mut lightmap_light_scale = 1.0;
				if let Some(light) = get_entity_f32(map_entity, map, "light")
				{
					lightmap_light_scale = 0.0;
					let mut color = [1.0, 1.0, 1.0];
					if let Some(entity_color_str) = get_entity_key_value(map_entity, map, "color")
					{
						if let Ok(entity_color) = map_file_common::parse_vec3(entity_color_str)
						{
							color[0] = (entity_color.x / 255.0).max(0.0).min(1.0);
							color[1] = (entity_color.y / 255.0).max(0.0).min(1.0);
							color[2] = (entity_color.z / 255.0).max(0.0).min(1.0);
						}
					}

					light_add = color.map(|c| c * light);
				}

				ecs.spawn((Decal {
					position: origin,
					rotation: get_entity_rotation(map_entity, map),
					scale: size,
					texture,
					blending_mode: get_entity_blending_mode(map_entity, map),
					lightmap_light_scale,
					light_add,
				},));
			}
		},
		Some("light_wall_oil_lamp") =>
		{
			if let Some(origin) = get_entity_origin(map_entity, map)
			{
				let position = origin;
				let rotation = get_entity_rotation(map_entity, map);

				ecs.spawn((
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
						model: resources_manager.get_model("wall_oil_lamp.iqm"),
						texture: resources_manager.get_texture_lite("wall_oil_lamp.png"),
						blending_mode: material::BlendingMode::None,
						lighting: ModelLighting::Default,
						flags: ModelEntityDrawFlags::empty(),
						ordering_custom_bbox: None,
					},
				));

				let sprite_texture = resources_manager.get_texture_lite("small_flame.png");
				let scale = 0.5;

				let texture_mip0 = &sprite_texture[0];
				let sprite_radius = 0.25 *
					scale * ((texture_mip0.size[0] * texture_mip0.size[0] +
					texture_mip0.size[1] * texture_mip0.size[1]) as f32)
					.sqrt();

				ecs.spawn((Sprite {
					position: position + Vec3f::new(0.0, 0.0, 2.0),
					angle: 0.0,
					radius: sprite_radius,
					texture: sprite_texture,
					orientation: SpriteOrientation::AlignToZAxisFacingTowardsCamera,
					blending_mode: material::BlendingMode::Additive,
					light_scale: 0.0,
					light_add: [8.0; 3],
				},));
			}
		},
		// Process unknown entities with submodels as "func_detal", but ignore triggers.
		Some("func_detail") | _ =>
		{
			// Spawn non-moving static entity.
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len() && !class_name.unwrap_or("").starts_with("trigger")
			{
				let bbox = bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]);

				let entity = ecs.spawn((SubmodelEntityWithIndex {
					index,
					submodel_entity: SubmodelEntity {
						position: bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]).get_center(),
						rotation: QuaternionF::one(),
					},
				},));
				ecs.insert_one(
					entity,
					physics.add_submodel_object(entity, &bbox, &Vec3f::zero(), &QuaternionF::one()),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
	}
}

pub fn spawn_player(
	ecs: &mut hecs::World,
	physics: &mut TestGamePhysics,
	resources_manager: &mut ResourcesManager,
	map: &bsp_map_compact::BSPMap,
) -> hecs::Entity
{
	let mut spawn_entity = None;
	for classname in [
		"info_player_start",
		"info_player_start2",
		"info_player_coop",
		"info_player_deathmatch",
	]
	{
		spawn_entity = find_first_entity_of_given_class(map, classname);
		if spawn_entity.is_some()
		{
			break;
		}
	}

	let player_entity = ecs.spawn((
		PlayerComponent {
			view_model_entity: hecs::Entity::DANGLING,
			flashlight_entity: hecs::Entity::DANGLING,
		},
		LocationComponent {
			position: Vec3f::zero(),
			rotation: QuaternionF::one(),
		},
		PlayerControllerLocationComponent {},
		TeleportableComponent { destination: None },
	));

	let mut rotation_controller = CameraRotationController::new();
	let mut position_source = PlayerPositionSource::Noclip(Vec3f::zero());
	if let Some(spawn_entity) = spawn_entity
	{
		if let Some(origin) = get_entity_origin(spawn_entity, map)
		{
			position_source = PlayerPositionSource::Phys(create_player_phys_object(physics, player_entity, &origin));
		}

		let angle_rad = get_entity_angle_rad(spawn_entity, map);
		rotation_controller.set_angles(angle_rad - 0.5 * std::f32::consts::PI, 0.0, 0.0);
	}

	ecs.insert_one(
		player_entity,
		PlayerControllerComponent {
			rotation_controller,
			position_source,
		},
	)
	.ok();

	spawn_player_shadow(ecs, resources_manager, player_entity);

	player_entity
}

fn spawn_player_shadow(ecs: &mut hecs::World, resources_manager: &mut ResourcesManager, player_entity: hecs::Entity)
{
	let position = Vec3f::zero();
	let rotation = QuaternionF::one();

	ecs.spawn((
		LocationComponent { position, rotation },
		OtherEntityLocationComponent {
			entity: player_entity,
			relative_position: Vec3f::new(0.0, 0.0, -28.0),
			relative_rotation: QuaternionF::from_angle_y(Rad(std::f32::consts::PI * 0.5)),
		},
		DecalLocationLinkComponent {},
		Decal {
			position,
			rotation,
			scale: Vec3f::new(32.0, 32.0, 32.0),
			// Shadow blob is totally-black textue with variable alpha.
			// So, use no lighting for it and alpha-blending in otder to darken polygons in shadow.
			texture: resources_manager.get_texture_lite("shadow_blob.png"),
			blending_mode: material::BlendingMode::AlphaBlend,
			lightmap_light_scale: 0.0,
			light_add: [0.0; 3],
		},
	));
}

pub fn create_player_phys_object(
	physics: &mut TestGamePhysics,
	player_entity: hecs::Entity,
	position: &Vec3f,
) -> ObjectHandle
{
	// Use same dimensions of player as in Quake.
	physics.add_character_object(player_entity, position, 31.0, 56.0)
}

fn add_entity_common_components(
	ecs: &mut hecs::World,
	map: &bsp_map_compact::BSPMap,
	map_entity: &bsp_map_compact::Entity,
	entity: hecs::Entity,
)
{
	if let Some(target) = get_entity_key_value(map_entity, map, "target")
	{
		ecs.insert_one(
			entity,
			TargetNameComponent {
				name: target.to_string(),
			},
		)
		.ok();
	}

	if let Some(targetname) = get_entity_key_value(map_entity, map, "targetname")
	{
		ecs.insert_one(
			entity,
			NamedTargetComponent {
				name: targetname.to_string(),
			},
		)
		.ok();
	}
}

fn prepare_linked_doors(ecs: &mut hecs::World, map: &bsp_map_compact::BSPMap)
{
	// Find touching doors groups.
	let mut master_doors = std::collections::HashMap::<hecs::Entity, (BBox, Vec<hecs::Entity>)>::new();
	let mut slave_doors_set = std::collections::HashSet::<hecs::Entity>::new();

	for (id0, (_door_component0, submodel_entity_with_index_component0)) in
		ecs.query::<(&DoorComponent, &SubmodelEntityWithIndex)>().iter()
	{
		if slave_doors_set.contains(&id0)
		{
			continue;
		}
		let mut bbox =
			bsp_map_compact::get_submodel_bbox(map, &map.submodels[submodel_entity_with_index_component0.index]);

		let mut slave_doors = Vec::new();

		for (id1, (_door_component1, submodel_entity_with_index_component1)) in
			ecs.query::<(&DoorComponent, &SubmodelEntityWithIndex)>().iter()
		{
			if id0 == id1
			{
				continue;
			}
			if slave_doors_set.contains(&id0)
			{
				continue;
			}

			let bbox1 =
				bsp_map_compact::get_submodel_bbox(map, &map.submodels[submodel_entity_with_index_component1.index]);
			if bbox.touches_or_intersects(&bbox1)
			{
				bbox.extend(&bbox1);
				slave_doors.push(id1);
				slave_doors_set.insert(id1);
			}
		}

		master_doors.insert(id0, (bbox, slave_doors));
	}

	// Spawn triggers for master doors only.
	// Slave doors will be automatically activated together with master doors.
	for (id, (bbox, slave_doors)) in master_doors.drain()
	{
		let mut q = ecs
			.query_one::<(&mut DoorComponent, Option<&NamedTargetComponent>)>(id)
			.unwrap();
		let (door_component, named_target_component) = q.get().unwrap();

		door_component.slave_doors = slave_doors;

		if named_target_component.is_some()
		{
			// This door is activated by some other trigger.
			continue;
		}
		drop(q);

		let bbox_increase = Vec3f::new(60.0, 60.0, 8.0);
		let trigger_bbox = BBox::from_min_max(bbox.min - bbox_increase, bbox.max + bbox_increase);

		ecs.spawn((
			TouchTriggerComponent { bbox: trigger_bbox },
			TriggerSingleTargetComponent { target: id },
		));
	}
}

// Returns normalized vector.
fn get_entity_move_direction(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> Vec3f
{
	if let Some(angle) = get_entity_key_value(entity, map, "angle")
	{
		if let Ok(num) = angle.parse::<f32>()
		{
			if num == -1.0
			{
				return Vec3f::new(0.0, 0.0, 1.0);
			}
			if num == -2.0
			{
				return Vec3f::new(0.0, 0.0, -1.0);
			}
			let angle_rad = num * (std::f32::consts::PI / 180.0);
			return Vec3f::new(angle_rad.cos(), angle_rad.sin(), 0.0);
		}
	}

	// TODO - support "angles".

	// Return something.
	Vec3f::unit_x()
}

fn find_first_entity_of_given_class<'a>(
	map: &'a bsp_map_compact::BSPMap,
	classname: &str,
) -> Option<&'a bsp_map_compact::Entity>
{
	for entity in &map.entities
	{
		if get_entity_classname(entity, map) == Some(classname)
		{
			return Some(entity);
		}
	}

	None
}

const TO_RAD: f32 = std::f32::consts::PI / 180.0;

fn get_entity_angle_rad(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> f32
{
	get_entity_f32(entity, map, "angle").unwrap_or(0.0) * TO_RAD
}

fn get_entity_rotation(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> QuaternionF
{
	if let Some(angles_str) = get_entity_key_value(entity, map, "angles")
	{
		if let Ok(angles) = map_file_common::parse_vec3(angles_str)
		{
			// Angles in Quake and Quake editors (like TrenchBroom) are in order YZX.
			return QuaternionF::from_angle_z(Rad(angles.y * TO_RAD)) *
				QuaternionF::from_angle_y(Rad(angles.x * TO_RAD)) *
				QuaternionF::from_angle_x(Rad(angles.z * TO_RAD));
		}
	}

	QuaternionF::from_angle_z(Rad(get_entity_angle_rad(entity, map)))
}

fn get_entity_f32(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap, key: &str) -> Option<f32>
{
	get_entity_key_value(entity, map, key).unwrap_or("").parse::<f32>().ok()
}

fn get_entity_origin(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> Option<Vec3f>
{
	if let Some(origin_str) = get_entity_key_value(entity, map, "origin")
	{
		map_file_common::parse_vec3(origin_str).ok()
	}
	else
	{
		None
	}
}

fn get_entity_blending_mode(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> material::BlendingMode
{
	// TODO - fix this mess, use string representations of properties, instead of meaningless numbers.
	if let Some(blending_mode) = get_entity_f32(entity, map, "blending_mode")
	{
		match blending_mode as u32
		{
			0 => material::BlendingMode::None,
			1 => material::BlendingMode::Average,
			2 => material::BlendingMode::Additive,
			3 => material::BlendingMode::AlphaTest,
			4 => material::BlendingMode::AlphaBlend,
			_ => material::BlendingMode::None,
		}
	}
	else
	{
		material::BlendingMode::None
	}
}

fn get_entity_classname<'a>(entity: &bsp_map_compact::Entity, map: &'a bsp_map_compact::BSPMap) -> Option<&'a str>
{
	get_entity_key_value(entity, map, "classname")
}

fn get_entity_key_value<'a>(
	entity: &bsp_map_compact::Entity,
	map: &'a bsp_map_compact::BSPMap,
	key: &str,
) -> Option<&'a str>
{
	for key_value in &map.key_value_pairs
		[entity.first_key_value_pair as usize .. (entity.first_key_value_pair + entity.num_key_value_pairs) as usize]
	{
		let actual_key = bsp_map_compact::get_map_string(key_value.key, map);
		if actual_key == key
		{
			return Some(bsp_map_compact::get_map_string(key_value.value, map));
		}
	}

	None
}

fn get_portal_texture(
	resources_manager: &mut ResourcesManager,
	map: &bsp_map_compact::BSPMap,
	texture_index: u32,
) -> Option<ViewPortalTexture>
{
	if let Some(material) = resources_manager
		.get_materials()
		.get(bsp_map_compact::get_texture_string(
			&map.textures[texture_index as usize],
		))
	{
		if let Some(portal_texture) = material.extra.get("portal_texture")
		{
			if let Some(portal_texture_str) = portal_texture.as_str()
			{
				let blending_mode = if let Some(portal_blending_mode) = material.extra.get("portal_blending_mode")
				{
					serde_json::from_value(portal_blending_mode.clone()).ok()
				}
				else
				{
					None
				};

				return Some(ViewPortalTexture {
					texture: resources_manager.get_texture_lite(portal_texture_str),
					blending_mode: blending_mode.unwrap_or(material::BlendingMode::Average),
					light_scale: 1.0,
					light_add: [0.0; 3],
				});
			}
		}
	}

	None
}
