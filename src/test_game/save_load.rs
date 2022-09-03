use super::components::*;

pub fn save_world(ecs: &hecs::World, file_path: &std::path::Path)
{
	let file = std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(file_path)
		.unwrap();

	let mut context = SerializeContext {};
	let mut serializer = serde_json::Serializer::pretty(file);
	hecs::serialize::row::serialize(ecs, &mut context, &mut serializer).unwrap();
}

struct SerializeContext {}

impl SerializeContext
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

impl hecs::serialize::row::SerializeContext for SerializeContext
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

		// TODO - add drawable components.

		map.end()
	}

	fn component_count(&self, entity: hecs::EntityRef) -> Option<usize>
	{
		Some(entity.len())
	}
}
