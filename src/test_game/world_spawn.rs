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
	match get_entity_classname(map_entity, map)
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
								index,
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
								index,
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
								index,
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
							phys_handle: physics.add_submodel_object(entity, index, &Vec3f::zero(), &rotation),
						},
						// Update draw model location, using location component.
						SubmodelEntityWithIndexLocationLinkComponent {},
					),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
		Some("misc_model") =>
		{
			if let (Some(model_file_name), Some(origin)) = (
				get_entity_key_value(map_entity, map, "model"),
				get_entity_origin(map_entity, map),
			)
			{
				let model_file_name = model_file_name.to_string();

				let model = resources_manager.get_model(&model_file_name);

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
				let rotation = QuaternionF::from_angle_z(Rad(get_entity_angle_rad(map_entity, map)));

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
						blending_mode: material::BlendingMode::None,
						lighting: ModelLighting::Default,
						is_view_model: false,
						ordering_custom_bbox: None,
					},
				));
			}
		},
		// Process unknown entities with submodels as "func_detal".
		Some("func_detail") | _ =>
		{
			// Spawn non-moving static entity.
			let index = map_entity.submodel_index as usize;
			if index < map.submodels.len()
			{
				let entity = ecs.spawn((SubmodelEntityWithIndex {
					index,
					submodel_entity: SubmodelEntity {
						position: bsp_map_compact::get_submodel_bbox(map, &map.submodels[index]).get_center(),
						rotation: QuaternionF::one(),
					},
				},));
				ecs.insert_one(
					entity,
					physics.add_submodel_object(entity, index, &Vec3f::zero(), &QuaternionF::one()),
				)
				.ok();

				add_entity_common_components(ecs, map, map_entity, entity);
			}
		},
	}
}

pub fn spawn_player(ecs: &mut hecs::World, physics: &mut TestGamePhysics, map: &bsp_map_compact::BSPMap)
	-> hecs::Entity
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
		},
		LocationComponent {
			position: Vec3f::zero(),
			rotation: QuaternionF::one(),
		},
		PlayerControllerLocationComponent {},
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

	player_entity
}

pub fn create_player_phys_object(
	physics: &mut TestGamePhysics,
	player_entity: hecs::Entity,
	position: &Vec3f,
) -> ObjectHandle
{
	// Use same dimensions of player as in Quake.
	physics.add_character_object(player_entity, &position, 31.0, 56.0)
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
	return Vec3f::unit_x();
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

fn get_entity_angle_rad(entity: &bsp_map_compact::Entity, map: &bsp_map_compact::BSPMap) -> f32
{
	get_entity_f32(entity, map, "angle").unwrap_or(0.0) * (std::f32::consts::PI / 180.0)
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