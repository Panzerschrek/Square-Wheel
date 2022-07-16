use super::{config, resources_manager_config::*, textures::*, triangle_model, triangle_model_md3};
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
	default_material: Material,

	models: ResourcesMap<triangle_model::TriangleModel>,
	images: ResourcesMap<image::Image>,
	material_textures: ResourcesMap<TextureWithMips>,
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
			default_material: Material::default(),
			models: ResourcesMap::new(),
			images: ResourcesMap::new(),
			material_textures: ResourcesMap::new(),
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

	pub fn get_material_texture(&mut self, key: &ResourceKey) -> SharedResourcePtr<TextureWithMips>
	{
		if let Some(p) = self.material_textures.get(key)
		{
			return p.clone();
		}

		let material = self.materials.get(key).unwrap_or(&self.default_material);

		let texture_with_mips = load_texture(material, &self.config.textures_path);

		let ptr = Arc::new(texture_with_mips);
		self.material_textures.insert(key.clone(), ptr.clone());

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

fn load_texture(material: &Material, textures_path: &str) -> TextureWithMips
{
	let diffuse = if let Some(image) = load_image(
		&material.diffuse.clone().unwrap_or_else(|| String::new()),
		textures_path,
	)
	{
		image
	}
	else
	{
		image::make_stub()
	};

	let normals = if let Some(normal_map_texture) = &material.normal_map
	{
		load_image(&normal_map_texture.clone(), textures_path)
	}
	else
	{
		None
	};

	let roughness_map = if let Some(roughness_map_texture) = &material.roughness_map
	{
		load_image(&roughness_map_texture.clone(), textures_path)
	}
	else
	{
		None
	};

	let mip0 = make_texture(diffuse, normals, material.roughness, roughness_map, material.is_metal);

	build_texture_mips(mip0)
}

fn load_image(file_name: &str, textures_path: &str) -> Option<image::Image>
{
	let mut path = PathBuf::from(textures_path);
	path.push(file_name);
	image::load(&path)
}
