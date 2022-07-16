use super::{config, resources_manager_config::*, triangle_model, triangle_model_md3};
use common::{image, material::*};
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, Mutex},
};

// Resources loader class with internal caching.
pub struct ResourcesManager
{
	config: ResourcesManagerConfig,

	materials: SharedResourcePtr<MaterialsMap>,
	models: ResourcesMap<triangle_model::TriangleModel>,
	images: ResourcesMap<image::Image>,
}

pub type ResourcesManagerSharedPtr = Arc<Mutex<ResourcesManager>>;

pub type SharedResourcePtr<T> = Arc<T>;

pub type ResourceKey = String;

type ResourcesMap<T> = HashMap<String, SharedResourcePtr<T>>;

impl ResourcesManager
{
	pub fn new(app_config: config::ConfigSharedPtr) -> ResourcesManagerSharedPtr
	{
		let config_parsed = ResourcesManagerConfig::from_app_config(&app_config);
		config_parsed.update_app_config(&app_config); // Update JSON with struct fields.

		let materials = Arc::new(load_materials(&PathBuf::from(config_parsed.materials_path.clone())));

		Arc::new(Mutex::new(Self {
			config: config_parsed,
			materials,
			models: ResourcesMap::new(),
			images: ResourcesMap::new(),
		}))
	}

	pub fn get_materials(&mut self) -> SharedResourcePtr<MaterialsMap>
	{
		self.materials.clone()
	}

	pub fn get_model(&mut self, key: &ResourceKey) -> SharedResourcePtr<triangle_model::TriangleModel>
	{
		if let Some(p) = self.models.get(key)
		{
			return p.clone();
		}

		// TODO - use dummy instead of "unwrap".
		// TODO - specify models root path.
		let model = triangle_model_md3::load_model_md3(&PathBuf::from(key))
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

		let image = load_image(&key, &self.config.textures_path).unwrap_or_else(image::make_stub);

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

fn load_image(file_name: &str, textures_path: &str) -> Option<image::Image>
{
	let mut path = PathBuf::from(textures_path);
	path.push(file_name);
	image::load(&path)
}
