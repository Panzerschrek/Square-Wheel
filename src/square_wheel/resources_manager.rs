use super::{triangle_model, triangle_model_md3};
use common::image;
use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

// Resources loader class with internal caching.
pub struct ResourcesManager
{
	models: ResourcesMap<triangle_model::TriangleModel>,
	images: ResourcesMap<image::Image>,
}

pub type ResourcesManagerSharedPtr = Arc<Mutex<ResourcesManager>>;

pub type SharedResourcePtr<T> = Arc<T>;

pub type ResourceKey = String;

type ResourcesMap<T> = HashMap<String, SharedResourcePtr<T>>;

impl ResourcesManager
{
	pub fn new() -> ResourcesManagerSharedPtr
	{
		Arc::new(Mutex::new(Self {
			models: ResourcesMap::new(),
			images: ResourcesMap::new(),
		}))
	}

	pub fn get_model(&mut self, key: &ResourceKey) -> SharedResourcePtr<triangle_model::TriangleModel>
	{
		if let Some(p) = self.models.get(key)
		{
			return p.clone();
		}

		// TODO - use dummy instead of "unwrap".
		// TODO - specify models root path.
		let model = triangle_model_md3::load_model_md3(&std::path::PathBuf::from(key))
			.unwrap()
			.unwrap();

		let ptr = Arc::new(model);
		self.models.insert(key.clone(), ptr.clone());

		ptr
	}

	pub fn get_image(&mut self, key: &ResourceKey) -> SharedResourcePtr<image::Image>
	{
		if let Some(p) = self.images.get(key)
		{
			return p.clone();
		}

		// TODO - use dummy instead of "unwrap".
		// TODO - specify images root path.
		let image = image::load(&std::path::PathBuf::from(key)).unwrap();

		let ptr = Arc::new(image);
		self.images.insert(key.clone(), ptr.clone());

		ptr
	}

	pub fn clear_cache(&mut self)
	{
		// Remove all resources that are stored only inside cache.
		remove_unused_resource_map_entries(&mut self.models);
		remove_unused_resource_map_entries(&mut self.images);
	}
}

fn remove_unused_resource_map_entries<T>(map: &mut ResourcesMap<T>)
{
	map.retain(|_k, v| Arc::strong_count(v) > 1);
}
