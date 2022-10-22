use super::{components::*, frame_info::*, resources_manager, test_game_physics, textures, triangle_model};
use serde::{Deserialize, Serialize};
use square_wheel_lib::common::{bbox::*, material, math_types::*};
use std::{
	collections::HashMap,
	fs::{File, OpenOptions},
	io::{Read, Seek, SeekFrom, Write},
	path::Path,
	sync::Arc,
};

pub fn save(
	ecs: &hecs::World,
	physics: &test_game_physics::TestGamePhysics,
	game_time: f32,
	player_entity: hecs::Entity,
	file_path: &Path,
	resources_manager: &resources_manager::ResourcesManager,
) -> Option<()>
{
	let mut file = OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)
		.ok()?;

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
	file.write_all(header_bytes).ok()?;

	header.common_data_offset = file.stream_position().ok()? as u32;
	save_common_data(game_time, player_entity, &mut file)?;

	header.ecs_data_offset = file.stream_position().ok()? as u32;
	let shared_resources = save_ecs(ecs, &mut file)?;

	header.resources_data_offset = file.stream_position().ok()? as u32;
	save_shared_resources(&shared_resources, resources_manager, &mut file)?;

	header.physics_data_offset = file.stream_position().ok()? as u32;
	save_physics(physics, &mut file)?;

	// Write header again to update offsets.
	file.seek(SeekFrom::Start(0)).ok()?;
	file.write_all(header_bytes).ok()?;
	file.sync_data().ok()?;

	Some(())
}

pub struct LoadResult
{
	pub ecs: hecs::World,
	pub physics: test_game_physics::TestGamePhysics,
	pub game_time: f32,
	pub player_entity: hecs::Entity,
}

pub fn load(file_path: &Path, resources_manager: &mut resources_manager::ResourcesManager) -> Option<LoadResult>
{
	let mut file = OpenOptions::new()
		.read(true)
		.write(false)
		.create(false)
		.open(file_path)
		.ok()?;

	let header_size = std::mem::size_of::<SaveHeader>();
	let mut header = unsafe { std::mem::zeroed::<SaveHeader>() };
	let header_bytes =
		unsafe { std::slice::from_raw_parts_mut((&mut header) as *mut SaveHeader as *mut u8, header_size) };

	if file.read(header_bytes).ok()? != header_size
	{
		println!("Can't read save header");
		return None;
	}

	if header.id != SAVE_ID
	{
		println!("File is not a valid save");
		return None;
	}
	if header.version != SAVE_VERSION
	{
		println!(
			"Can't load incompatible save version: {}, expected {}",
			header.version, SAVE_VERSION
		);
		return None;
	}

	file.seek(SeekFrom::Start(header.common_data_offset as u64)).ok()?;
	let common_data = load_common_data(&mut file)?;

	file.seek(SeekFrom::Start(header.resources_data_offset as u64)).ok()?;
	let shared_resources = load_shared_resources(resources_manager, &mut file)?;

	file.seek(SeekFrom::Start(header.ecs_data_offset as u64)).ok()?;
	let ecs = load_ecs(&shared_resources, &mut file)?;

	file.seek(SeekFrom::Start(header.physics_data_offset as u64)).ok()?;
	let physics = load_physics(&mut file)?;

	Some(LoadResult {
		ecs,
		physics,
		game_time: common_data.game_time,
		player_entity: common_data.player_entity,
	})
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

fn save_common_data(game_time: f32, player_entity: hecs::Entity, file: &mut File) -> Option<()>
{
	bincode::serialize_into(
		file,
		&CommonData {
			game_time,
			player_entity,
		},
	)
	.ok()
}

fn load_common_data(file: &mut File) -> Option<CommonData>
{
	bincode::deserialize_from(file).ok()
}

#[derive(Serialize, Deserialize)]
struct CommonData
{
	game_time: f32,
	player_entity: hecs::Entity,
}

fn save_ecs(ecs: &hecs::World, file: &mut File) -> Option<SharedResources>
{
	let mut shared_resources = SharedResources::default();

	let mut context = SerializeContext {
		shared_resources: &mut shared_resources,
	};

	let mut serializer = bincode::Serializer::new(file, bincode::DefaultOptions::new());
	hecs::serialize::row::serialize(ecs, &mut context, &mut serializer).ok()?;

	Some(shared_resources)
}

fn load_ecs(shared_resources: &SharedResources, file: &mut File) -> Option<hecs::World>
{
	let mut context = DeserializeContext { shared_resources };

	let mut deserializer = bincode::Deserializer::with_reader(file, bincode::DefaultOptions::new());
	hecs::serialize::row::deserialize(&mut context, &mut deserializer).ok()
}

fn save_shared_resources(
	shared_resources: &SharedResources,
	resources_manager: &resources_manager::ResourcesManager,
	file: &mut File,
) -> Option<()>
{
	let mut r = ResourcesForSerialization::default();

	for (key, model) in &shared_resources.models
	{
		r.models.insert(
			*key,
			if let Some(name) = resources_manager.get_model_name(model)
			{
				ResourceForSerialization::Named(name.to_string())
			}
			else
			{
				ResourceForSerialization::Direct((**model).clone())
			},
		);
	}

	for (key, texture) in &shared_resources.lite_textures
	{
		r.lite_textures.insert(
			*key,
			if let Some(name) = resources_manager.get_texture_lite_name(texture)
			{
				ResourceForSerialization::Named(name.to_string())
			}
			else
			{
				ResourceForSerialization::Direct((**texture).clone())
			},
		);
	}

	bincode::serialize_into(file, &r).ok()
}

fn load_shared_resources(
	resources_manager: &mut resources_manager::ResourcesManager,
	file: &mut File,
) -> Option<SharedResources>
{
	let mut shared_resources_serialized: ResourcesForSerialization = bincode::deserialize_from(file).ok()?;

	let mut shared_resources = SharedResources::default();

	for (key, model_resource) in &mut shared_resources_serialized.models.drain()
	{
		let model = match model_resource
		{
			ResourceForSerialization::Named(name) => resources_manager.get_model(&name),
			ResourceForSerialization::Direct(r) => Arc::new(r),
		};
		shared_resources.models.insert(key, model);
	}

	for (key, texture_resource) in &mut shared_resources_serialized.lite_textures.drain()
	{
		let model = match texture_resource
		{
			ResourceForSerialization::Named(name) => resources_manager.get_texture_lite(&name),
			ResourceForSerialization::Direct(r) => Arc::new(r),
		};
		shared_resources.lite_textures.insert(key, model);
	}

	Some(shared_resources)
}

#[derive(Default, Serialize, Deserialize)]
struct ResourcesForSerialization
{
	models: HashMap<ResourceSerializationKey, ResourceForSerialization<triangle_model::TriangleModel>>,
	lite_textures: HashMap<ResourceSerializationKey, ResourceForSerialization<textures::TextureLiteWithMips>>,
}

#[derive(Serialize, Deserialize)]
enum ResourceForSerialization<T>
{
	// Save only name, request resource from ResourcesManager during deserialization.
	Named(String),
	// Save resource directly.
	// It is possible for generated resources or othrer resources, obtained not via ResourcesManager.
	Direct(T),
}

fn save_physics(physics: &test_game_physics::TestGamePhysics, file: &mut File) -> Option<()>
{
	bincode::serialize_into(file, physics).ok()
}

fn load_physics(file: &mut File) -> Option<test_game_physics::TestGamePhysics>
{
	bincode::deserialize_from(file).ok()
}

struct SerializeContext<'a>
{
	shared_resources: &'a mut SharedResources,
}

impl<'a> SerializeContext<'a>
{
	fn try_serialize_component<
		'de,
		T: hecs::Component + serde::Serialize + serde::Deserialize<'de>,
		S: serde::ser::SerializeMap,
	>(
		&self,
		entity: hecs::EntityRef,
		map: &mut S,
	) -> Result<Option<S::Ok>, S::Error>
	{
		if let Some(c) = entity.get::<&T>()
		{
			map.serialize_entry(get_component_name::<T>(), &*c)?;
		}

		Ok(None)
	}
}

impl<'a> hecs::serialize::row::SerializeContext for SerializeContext<'a>
{
	fn serialize_entity<S: serde::ser::SerializeMap>(
		&mut self,
		entity: hecs::EntityRef,
		mut map: S,
	) -> Result<S::Ok, S::Error>
	{
		// Do not forget to add new components here!

		self.try_serialize_component::<TestModelComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TestDecalComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TestSpriteComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TestLightComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TestProjectileComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<LocationComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TimedDespawnComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<PlayerControllerLocationComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<PhysicsLocationComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<LocationKinematicPhysicsObjectComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<OtherEntityLocationComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<PlayerControllerCameraLocationComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<ModelEntityLocationLinkComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<SubmodelEntityWithIndexLocationLinkComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<DecalLocationLinkComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<DynamicLightLocationLinkComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<SimpleAnimationComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<PlayerComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<PlayerControllerComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<TouchTriggerComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TriggerSingleTargetComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TargetNameComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<NamedTargetComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<WaitComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<EntityActivationComponent, S>(entity, &mut map)?;

		self.try_serialize_component::<PlateComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<DoorComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<ButtonComponent, S>(entity, &mut map)?;
		self.try_serialize_component::<TrainComponent, S>(entity, &mut map)?;

		// Non-special drawable components.
		self.try_serialize_component::<SubmodelEntityWithIndex, S>(entity, &mut map)?;
		self.try_serialize_component::<DynamicLight, S>(entity, &mut map)?;

		// Perform serialization of components with shared resources, using proxy structs with identical content but changed resource fields.
		// Collect shared resources and serialize them later.

		if let Some(c) = entity.get::<&ModelEntity>()
		{
			let proxy = ModelEntityProxy::new(
				&*c,
				&mut self.shared_resources.models,
				&mut self.shared_resources.lite_textures,
			);
			map.serialize_entry(get_component_name::<ModelEntityProxy>(), &proxy)?;
		}

		if let Some(c) = entity.get::<&Decal>()
		{
			let proxy = DecalProxy::new(&*c, &mut self.shared_resources.lite_textures);
			map.serialize_entry(get_component_name::<DecalProxy>(), &proxy)?;
		}

		if let Some(c) = entity.get::<&Sprite>()
		{
			let proxy = SpriteProxy::new(&*c, &mut self.shared_resources.lite_textures);
			map.serialize_entry(get_component_name::<SpriteProxy>(), &proxy)?;
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
	fn try_deserialize_component<'de, T: hecs::Component + Deserialize<'de>, M: serde::de::MapAccess<'de>>(
		&'a self,
		key: &str,
		map: &mut M,
		entity: &mut hecs::EntityBuilder,
	) -> Result<(), M::Error>
	{
		if key == get_component_name::<T>()
		{
			entity.add::<T>(map.next_value()?);
		}

		Ok(())
	}
}

impl<'a> hecs::serialize::row::DeserializeContext for DeserializeContext<'a>
{
	fn deserialize_entity<'de, M: serde::de::MapAccess<'de>>(
		&mut self,
		mut map: M,
		entity: &mut hecs::EntityBuilder,
	) -> Result<(), M::Error>
	{
		while let Some(key) = map.next_key::<String>()?
		{
			// Do not forget to add new components here!

			self.try_deserialize_component::<TestModelComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TestDecalComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TestSpriteComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TestLightComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TestProjectileComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<LocationComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TimedDespawnComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<PlayerControllerLocationComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<PhysicsLocationComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<LocationKinematicPhysicsObjectComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<OtherEntityLocationComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<PlayerControllerCameraLocationComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<ModelEntityLocationLinkComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<SubmodelEntityWithIndexLocationLinkComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<DecalLocationLinkComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<DynamicLightLocationLinkComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<SimpleAnimationComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<PlayerComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<PlayerControllerComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<TouchTriggerComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TriggerSingleTargetComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TargetNameComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<NamedTargetComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<WaitComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<EntityActivationComponent, M>(&key, &mut map, entity)?;

			self.try_deserialize_component::<PlateComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<DoorComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<ButtonComponent, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<TrainComponent, M>(&key, &mut map, entity)?;

			// Non-special drawable components.
			self.try_deserialize_component::<SubmodelEntityWithIndex, M>(&key, &mut map, entity)?;
			self.try_deserialize_component::<DynamicLight, M>(&key, &mut map, entity)?;

			// Drawable components with shared resources.
			if key.as_str() == get_component_name::<ModelEntityProxy>()
			{
				if let Some(v) = map
					.next_value::<ModelEntityProxy>()?
					.try_to(&self.shared_resources.models, &self.shared_resources.lite_textures)
				{
					entity.add(v);
				}
			}
			if key.as_str() == get_component_name::<DecalProxy>()
			{
				if let Some(v) = map
					.next_value::<DecalProxy>()?
					.try_to(&self.shared_resources.lite_textures)
				{
					entity.add(v);
				}
			}
			if key.as_str() == get_component_name::<SpriteProxy>()
			{
				if let Some(v) = map
					.next_value::<SpriteProxy>()?
					.try_to(&self.shared_resources.lite_textures)
				{
					entity.add(v);
				}
			}
		}

		Ok(())
	}
}

fn get_component_name<'de, T: Deserialize<'de>>() -> &'static str
{
	// Use "unwrap" to catch early problems with broken components naming.
	serde_name::trace_name::<T>().unwrap()
}

#[derive(Default)]
struct SharedResources
{
	models: ResourcesMap<triangle_model::TriangleModel>,
	lite_textures: ResourcesMap<textures::TextureLiteWithMips>,
}

// Use hash map with pointer converted to integer as key because HashSet in not possible for Arc.
type ResourcesMap<T> = HashMap<ResourceSerializationKey, resources_manager::SharedResourcePtr<T>>;

// Use pointers as resources keys.
#[derive(Copy, Clone, Serialize, Deserialize, std::cmp::PartialEq, std::cmp::Eq, std::hash::Hash)]
struct ResourceSerializationKey(u64);

impl ResourceSerializationKey
{
	fn from_resource<T>(resource: &resources_manager::SharedResourcePtr<T>, resources: &mut ResourcesMap<T>) -> Self
	{
		let key = Self(Arc::as_ptr(resource) as usize as u64);

		if !resources.contains_key(&key)
		{
			resources.insert(key, resource.clone());
		}
		key
	}

	fn to_resource<T>(self, resources: &ResourcesMap<T>) -> Option<resources_manager::SharedResourcePtr<T>>
	{
		resources.get(&self).map(|x| x.clone())
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

#[derive(Serialize, Deserialize)]
struct SpriteProxy
{
	position: Vec3f,
	radius: f32,
	texture: ResourceSerializationKey,
	blending_mode: material::BlendingMode,
	orientation: SpriteOrientation,
}

impl SpriteProxy
{
	fn new(sprite: &Sprite, textures: &mut ResourcesMap<textures::TextureLiteWithMips>) -> Self
	{
		Self {
			position: sprite.position,
			radius: sprite.radius,
			texture: ResourceSerializationKey::from_resource(&sprite.texture, textures),
			blending_mode: sprite.blending_mode,
			orientation: sprite.orientation,
		}
	}

	fn try_to(&self, textures: &ResourcesMap<textures::TextureLiteWithMips>) -> Option<Sprite>
	{
		Some(Sprite {
			position: self.position,
			radius: self.radius,
			texture: self.texture.to_resource(textures)?,
			blending_mode: self.blending_mode,
			orientation: self.orientation,
		})
	}
}
