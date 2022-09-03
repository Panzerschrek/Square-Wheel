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

impl hecs::serialize::row::SerializeContext for SerializeContext
{
	fn serialize_entity<S>(&mut self, entity: hecs::EntityRef<'_>, map: S) -> Result<S::Ok, S::Error>
	where
		S: serde::ser::SerializeMap,
	{
		// TODO
		map.end()
	}

	fn component_count(&self, entity: hecs::EntityRef<'_>) -> Option<usize>
	{
		None
	}
}
