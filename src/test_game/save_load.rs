use super::{components::*, frame_info::*, resources_manager, test_game_physics, textures, triangle_model};
use serde::{Deserialize, Serialize, Serializer};
use square_wheel_lib::common::{bbox::*, material, math_types::*};
use std::{
	collections::HashMap,
	io::{Read, Seek, Write},
};

pub fn save(
	ecs: &hecs::World,
	physics: &test_game_physics::TestGamePhysics,
	game_time: f32,
	player_entity: hecs::Entity,
	file_path: &std::path::Path,
	resources_manager: &resources_manager::ResourcesManager,
) -> Result<(), std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)
		.unwrap();

	let mut header = SaveHeader {
		id: SAVE_ID,
		version: SAVE_VERSION,
		common_data_offset: 0,
		ecs_data_offset: 0,
		resources_data_offset: 0,
		physics_data_offset: 0,
	};

	// Write header first time to advance current file position.
	let header_bytes = unsafe {
		std::slice::from_raw_parts(
			(&header) as *const SaveHeader as *const u8,
			std::mem::size_of::<SaveHeader>(),
		)
	};
	file.write_all(header_bytes)?;

	save_common_data(game_time, player_entity, &mut file);
	header.common_data_offset = file.stream_position()? as u32;

	header.ecs_data_offset = file.stream_position()? as u32;
	let shared_resources = save_ecs(ecs, &mut file);

	header.resources_data_offset = file.stream_position()? as u32;
	save_shared_resources(&shared_resources, resources_manager, &mut file);

	header.physics_data_offset = file.stream_position()? as u32;
	save_physics(physics, &mut file);

	// Write header again to update offsets.
	file.seek(std::io::SeekFrom::Start(0))?;
	file.write_all(header_bytes)?;
	file.sync_data()?;

	Ok(())
}

pub fn load(file_path: &std::path::Path) -> Result<Option<hecs::World>, std::io::Error>
{
	let mut file = std::fs::OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)?;

	let header_size = std::mem::size_of::<SaveHeader>();
	let mut header = unsafe { std::mem::zeroed::<SaveHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut SaveHeader as *mut u8, header_size) };

	if file.read(header_bytes)? != header_size
	{
		println!("Can't read save header");
		return Ok(None);
	}

	if header.id != SAVE_ID
	{
		println!("File is not a valid save");
		return Ok(None);
	}
	if header.version != SAVE_VERSION
	{
		println!(
			"Can't load incompatible save version: {}, expected {}",
			header.version, SAVE_VERSION
		);
		return Ok(None);
	}

	file.seek(std::io::SeekFrom::Start(header.ecs_data_offset as u64))?;

	Ok(Some(load_ecs(&SharedResources::default(), &mut file)))
}

#[repr(C)]
struct SaveHeader
{
	id: [u8; 4],
	version: u32,
	common_data_offset: u32,
	ecs_data_offset: u32,
	resources_data_offset: u32,
	physics_data_offset: u32,
}

const SAVE_ID: [u8; 4] = ['S' as u8, 'q' as u8, 'w' as u8, 'S' as u8];
const SAVE_VERSION: u32 = 1; // Change each time when format is changed!

fn save_common_data(game_time: f32, player_entity: hecs::Entity, file: &mut std::fs::File)
{
	serde_json::to_writer_pretty(
		file,
		&CommonData {
			game_time,
			player_entity,
		},
	);
}

#[derive(Serialize, Deserialize)]
struct CommonData
{
	game_time: f32,
	player_entity: hecs::Entity,
}

fn save_ecs(ecs: &hecs::World, file: &mut std::fs::File) -> SharedResources
{
	let mut shared_resources = SharedResources::default();

	let mut context = SerializeContext {
		shared_resources: &mut shared_resources,
	};

	// TODO - maybe use binary serialization instead?
	let mut serializer = serde_json::Serializer::pretty(file);
	hecs::serialize::row::serialize(ecs, &mut context, &mut serializer).unwrap();

	shared_resources
}

fn load_ecs(shared_resources: &SharedResources, file: &mut std::fs::File) -> hecs::World
{
	let mut context = DeserializeContext { shared_resources };

	// TODO - maybe use binary serialization instead?
	let mut deserializer = serde_json::Deserializer::new(serde_json::de::IoRead::new(file));
	hecs::serialize::row::deserialize(&mut context, &mut deserializer).unwrap()
}

fn save_shared_resources(
	shared_resources: &SharedResources,
	resources_manager: &resources_manager::ResourcesManager,
	file: &mut std::fs::File,
)
{
	let mut models = serde_json::Map::with_capacity(shared_resources.models.len());
	for (_, model) in &shared_resources.models
	{
		if let Some(name) = resources_manager.get_model_name(model)
		{
			models.insert(
				ResourceSerializationKey::from_resource_unchecked(model).as_string(),
				serde_json::Value::from(name),
			);
		}
	}

	let mut lite_textures = serde_json::Map::with_capacity(shared_resources.lite_textures.len());
	for (_, texture) in &shared_resources.lite_textures
	{
		if let Some(name) = resources_manager.get_texture_lite_name(texture)
		{
			lite_textures.insert(
				ResourceSerializationKey::from_resource_unchecked(texture).as_string(),
				serde_json::Value::from(name),
			);
		}
	}

	let mut map = serde_json::Map::with_capacity(2);
	map.insert("models".to_string(), serde_json::Value::from(models));
	map.insert("lite_textures".to_string(), serde_json::Value::from(lite_textures));

	serde_json::to_writer_pretty(file, &serde_json::Value::from(map));
}

fn save_physics(physics: &test_game_physics::TestGamePhysics, file: &mut std::fs::File)
{
	// Use binary format for physics, because it is too heavy.
	let mut serializer = serde_cbor::Serializer::new(serde_cbor::ser::IoWrite::new(file));
	serializer.serialize_some(physics);
}

struct SerializeContext<'a>
{
	shared_resources: &'a mut SharedResources,
}

impl<'a> SerializeContext<'a>
{
	fn try_serialize_component<T, S>(&self, entity: hecs::EntityRef, map: &mut S)
	where
		T: hecs::Component + serde::Serialize,
		S: serde::ser::SerializeMap,
	{
		if let Some(c) = entity.get::<&T>()
		{
			// TODO - hanlde errors properly.
			// TODO - use serde-name instead.
			let type_name = serde_type_name::type_name(&*c).unwrap();
			map.serialize_entry(type_name, &*c).unwrap();
		}
	}
}

impl<'a> hecs::serialize::row::SerializeContext for SerializeContext<'a>
{
	fn serialize_entity<S>(&mut self, entity: hecs::EntityRef<'_>, mut map: S) -> Result<S::Ok, S::Error>
	where
		S: serde::ser::SerializeMap,
	{
		self.try_serialize_component::<TestModelComponent, S>(entity, &mut map);
		self.try_serialize_component::<TestDecalComponent, S>(entity, &mut map);
		self.try_serialize_component::<TestLightComponent, S>(entity, &mut map);
		self.try_serialize_component::<TestProjectileComponent, S>(entity, &mut map);

		self.try_serialize_component::<LocationComponent, S>(entity, &mut map);
		self.try_serialize_component::<TimedDespawnComponent, S>(entity, &mut map);

		self.try_serialize_component::<PlayerControllerLocationComponent, S>(entity, &mut map);
		self.try_serialize_component::<PhysicsLocationComponent, S>(entity, &mut map);
		self.try_serialize_component::<LocationKinematicPhysicsObjectComponent, S>(entity, &mut map);
		self.try_serialize_component::<OtherEntityLocationComponent, S>(entity, &mut map);
		self.try_serialize_component::<PlayerControllerCameraLocationComponent, S>(entity, &mut map);

		self.try_serialize_component::<ModelEntityLocationLinkComponent, S>(entity, &mut map);
		self.try_serialize_component::<SubmodelEntityWithIndexLocationLinkComponent, S>(entity, &mut map);
		self.try_serialize_component::<DecalLocationLinkComponent, S>(entity, &mut map);
		self.try_serialize_component::<DynamicLightLocationLinkComponent, S>(entity, &mut map);

		self.try_serialize_component::<SimpleAnimationComponent, S>(entity, &mut map);

		self.try_serialize_component::<PlayerComponent, S>(entity, &mut map);
		self.try_serialize_component::<PlayerControllerComponent, S>(entity, &mut map);

		self.try_serialize_component::<TouchTriggerComponent, S>(entity, &mut map);
		self.try_serialize_component::<TriggerSingleTargetComponent, S>(entity, &mut map);
		self.try_serialize_component::<TargetNameComponent, S>(entity, &mut map);
		self.try_serialize_component::<NamedTargetComponent, S>(entity, &mut map);
		self.try_serialize_component::<WaitComponent, S>(entity, &mut map);
		self.try_serialize_component::<EntityActivationComponent, S>(entity, &mut map);

		self.try_serialize_component::<PlateComponent, S>(entity, &mut map);
		self.try_serialize_component::<DoorComponent, S>(entity, &mut map);
		self.try_serialize_component::<ButtonComponent, S>(entity, &mut map);
		self.try_serialize_component::<TrainComponent, S>(entity, &mut map);

		// Non-special drawable components.
		self.try_serialize_component::<SubmodelEntityWithIndex, S>(entity, &mut map);
		self.try_serialize_component::<DynamicLight, S>(entity, &mut map);

		// Perform serialization of components with shared resources, using proxy structs with identical content but changed resource fields.
		// Collect shared resources and serialize them later.

		if let Some(c) = entity.get::<&ModelEntity>()
		{
			// TODO - hanlde errors properly.
			let proxy = ModelEntityProxy::new(
				&*c,
				&mut self.shared_resources.models,
				&mut self.shared_resources.lite_textures,
			);
			let type_name = serde_type_name::type_name(&proxy).unwrap();
			map.serialize_entry(type_name, &proxy).unwrap();
		}

		if let Some(c) = entity.get::<&Decal>()
		{
			// TODO - hanlde errors properly.
			let proxy = DecalProxy::new(&*c, &mut self.shared_resources.lite_textures);
			let type_name = serde_type_name::type_name(&proxy).unwrap();
			map.serialize_entry(type_name, &proxy).unwrap();
		}

		map.end()
	}

	fn component_count(&self, entity: hecs::EntityRef) -> Option<usize>
	{
		Some(entity.len())
	}
}

struct DeserializeContext<'a>
{
	shared_resources: &'a SharedResources,
}

impl<'a> DeserializeContext<'a>
{
	fn try_deserialize_component<'de, T, M>(
		&'a self,
		key: &str,
		map: &mut M,
		entity: &mut hecs::EntityBuilder,
	) -> Result<(), M::Error>
	where
		T: hecs::Component + Deserialize<'de>,
		M: serde::de::MapAccess<'de>,
	{
		if Some(key) == serde_name::trace_name::<T>()
		{
			entity.add::<T>(map.next_value()?);
		}

		Ok(())
	}
}

impl<'a> hecs::serialize::row::DeserializeContext for DeserializeContext<'a>
{
	fn deserialize_entity<'de, M>(&mut self, mut map: M, entity: &mut hecs::EntityBuilder) -> Result<(), M::Error>
	where
		M: serde::de::MapAccess<'de>,
	{
		while let Some(key) = map.next_key::<String>()?
		{
			self.try_deserialize_component::<TestModelComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TestDecalComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TestLightComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TestProjectileComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<LocationComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TimedDespawnComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<PlayerControllerLocationComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<PhysicsLocationComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<LocationKinematicPhysicsObjectComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<OtherEntityLocationComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<PlayerControllerCameraLocationComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<ModelEntityLocationLinkComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<SubmodelEntityWithIndexLocationLinkComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<DecalLocationLinkComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<DynamicLightLocationLinkComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<SimpleAnimationComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<PlayerComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<PlayerControllerComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<TouchTriggerComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TriggerSingleTargetComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TargetNameComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<NamedTargetComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<WaitComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<EntityActivationComponent, M>(&key, &mut map, entity);

			self.try_deserialize_component::<PlateComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<DoorComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<ButtonComponent, M>(&key, &mut map, entity);
			self.try_deserialize_component::<TrainComponent, M>(&key, &mut map, entity);

			// Non-special drawable components.
			self.try_deserialize_component::<SubmodelEntityWithIndex, M>(&key, &mut map, entity);
			self.try_deserialize_component::<DynamicLight, M>(&key, &mut map, entity);

			// Drawable components with shared resources.
			// TODO - fix this.
			if Some(key.as_str()) == serde_name::trace_name::<ModelEntityProxy>()
			{
				map.next_value::<ModelEntityProxy>()?;
			}
			if Some(key.as_str()) == serde_name::trace_name::<DecalProxy>()
			{
				map.next_value::<DecalProxy>()?;
			}
		}

		Ok(())
	}
}

#[derive(Default)]
struct SharedResources
{
	models: ResourcesMap<triangle_model::TriangleModel>,
	lite_textures: ResourcesMap<textures::TextureLiteWithMips>,
}

// Use hash map with pointer as key because HashSet in not possible for Arc.
type ResourcesMap<T> = HashMap<*const T, resources_manager::SharedResourcePtr<T>>;

// Use pointers as resources keys.
#[derive(Copy, Clone, Serialize, Deserialize)]
struct ResourceSerializationKey(u64);

impl ResourceSerializationKey
{
	fn from_resource<T>(resource: &resources_manager::SharedResourcePtr<T>, resources: &mut ResourcesMap<T>) -> Self
	{
		let ptr = std::sync::Arc::as_ptr(resource);

		if !resources.contains_key(&ptr)
		{
			resources.insert(ptr, resource.clone());
		}

		Self(ptr as usize as u64)
	}

	fn from_resource_unchecked<T>(resource: &resources_manager::SharedResourcePtr<T>) -> Self
	{
		Self(std::sync::Arc::as_ptr(resource) as usize as u64)
	}

	fn as_string(self) -> String
	{
		format!("{}", self.0)
	}

	fn to_resource<T>(self, resources: &ResourcesMap<T>) -> Option<resources_manager::SharedResourcePtr<T>>
	{
		let ptr = self.0 as *const T;
		resources.get(&ptr).map(|x| x.clone())
	}
}

#[derive(Serialize, Deserialize)]
struct ModelEntityProxy
{
	position: Vec3f,
	rotation: QuaternionF,
	animation: AnimationPoint,
	model: ResourceSerializationKey,
	texture: ResourceSerializationKey,
	blending_mode: material::BlendingMode,
	lighting: ModelLighting,
	is_view_model: bool,
	ordering_custom_bbox: Option<BBox>,
}

impl ModelEntityProxy
{
	fn new(
		model_entity: &ModelEntity,
		models: &mut ResourcesMap<triangle_model::TriangleModel>,
		textures: &mut ResourcesMap<textures::TextureLiteWithMips>,
	) -> Self
	{
		Self {
			position: model_entity.position,
			rotation: model_entity.rotation,
			animation: model_entity.animation,
			model: ResourceSerializationKey::from_resource(&model_entity.model, models),
			texture: ResourceSerializationKey::from_resource(&model_entity.texture, textures),
			blending_mode: model_entity.blending_mode,
			lighting: model_entity.lighting,
			is_view_model: model_entity.is_view_model,
			ordering_custom_bbox: model_entity.ordering_custom_bbox,
		}
	}

	fn try_to(
		&self,
		models: &ResourcesMap<triangle_model::TriangleModel>,
		textures: &ResourcesMap<textures::TextureLiteWithMips>,
	) -> Option<ModelEntity>
	{
		Some(ModelEntity {
			position: self.position,
			rotation: self.rotation,
			animation: self.animation,
			model: self.model.to_resource(models)?,
			texture: self.texture.to_resource(textures)?,
			blending_mode: self.blending_mode,
			lighting: self.lighting,
			is_view_model: self.is_view_model,
			ordering_custom_bbox: self.ordering_custom_bbox,
		})
	}
}

#[derive(Serialize, Deserialize)]
struct DecalProxy
{
	position: Vec3f,
	rotation: QuaternionF,
	scale: Vec3f,
	texture: ResourceSerializationKey,
	blending_mode: material::BlendingMode,
	lightmap_light_scale: f32,
	light_add: [f32; 3],
}

impl DecalProxy
{
	fn new(decal: &Decal, textures: &mut ResourcesMap<textures::TextureLiteWithMips>) -> Self
	{
		Self {
			position: decal.position,
			rotation: decal.rotation,
			scale: decal.scale,
			texture: ResourceSerializationKey::from_resource(&decal.texture, textures),
			blending_mode: decal.blending_mode,
			lightmap_light_scale: decal.lightmap_light_scale,
			light_add: decal.light_add,
		}
	}

	fn try_to(&self, textures: &ResourcesMap<textures::TextureLiteWithMips>) -> Option<Decal>
	{
		Some(Decal {
			position: self.position,
			rotation: self.rotation,
			scale: self.scale,
			texture: self.texture.to_resource(textures)?,
			blending_mode: self.blending_mode,
			lightmap_light_scale: self.lightmap_light_scale,
			light_add: self.light_add,
		})
	}
}
