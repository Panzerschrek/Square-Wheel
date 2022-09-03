use super::{components::*, frame_info::*, resources_manager, textures, triangle_model};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use square_wheel_lib::common::{bbox::*, material, math_types::*};
use std::collections::HashMap;

pub fn save_world(
	ecs: &hecs::World,
	file_path: &std::path::Path,
	resources_manager: &resources_manager::ResourcesManager,
)
{
	let file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)
		.unwrap();

	let mut shared_resources = SharedResources::default();

	let mut context = SerializeContext {
		shared_resources: &mut shared_resources,
	};

	let mut serializer = serde_json::Serializer::pretty(file);
	hecs::serialize::row::serialize(ecs, &mut context, &mut serializer).unwrap();

	// TODO - fix this mess.
	// serde_json is stupid - it can't serialize map while serializing map.
	{
		let mut seq = serializer.serialize_map(Some(shared_resources.models.len())).unwrap();
		for (_, model) in &shared_resources.models
		{
			if let Some(name) = resources_manager.get_model_name(model)
			{
				seq.serialize_key(&ResourceSerializationKey::from_resource_unchecked(model));
				seq.serialize_value(name);
			}
		}
		seq.end();
	}
	{
		let mut seq = serializer
			.serialize_map(Some(shared_resources.lite_textures.len()))
			.unwrap();
		for (_, texture) in &shared_resources.lite_textures
		{
			if let Some(name) = resources_manager.get_texture_lite_name(texture)
			{
				seq.serialize_key(&ResourceSerializationKey::from_resource_unchecked(texture));
				seq.serialize_value(name);
			}
		}
		seq.end();
	}
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

	// Weapon or thing in player's hands.
	// Draw it always and after any other models.
	is_view_model: bool,

	// Use it to override bbox (in object-space) to improve models ordering.
	// For example, use bbox with size reduced relative to true model bbox.
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
