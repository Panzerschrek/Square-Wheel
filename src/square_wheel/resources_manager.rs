use super::{config, console::*, resources_manager_config::*, textures::*, triangle_model, triangle_model_md3};
use common::{bsp_map_compact::*, bsp_map_save_load::*, image, material::*};
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{Arc, Mutex},
};

// Resources loader class with internal caching.
pub struct ResourcesManager
{
	console: ConsoleSharedPtr,
	config: ResourcesManagerConfig,

	materials: SharedResourcePtr<MaterialsMap>,
	default_material: Material,

	// Cache single map. TODO - maybe cache more maps?
	last_map: Option<(String, SharedResourcePtr<BSPMap>)>,

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
	pub fn new(app_config: config::ConfigSharedPtr, console: ConsoleSharedPtr) -> ResourcesManagerSharedPtr
	{
		let config = ResourcesManagerConfig::from_app_config(&app_config);
		config.update_app_config(&app_config); // Update JSON with struct fields.

		let materials = SharedResourcePtr::new(load_materials(&PathBuf::from(config.materials_path.clone())));

		Arc::new(Mutex::new(Self {
			console,
			config,
			materials,
			default_material: Material::default(),
			last_map: None,
			models: ResourcesMap::new(),
			images: ResourcesMap::new(),
			material_textures: ResourcesMap::new(),
		}))
	}

	pub fn get_materials(&mut self) -> SharedResourcePtr<MaterialsMap>
	{
		self.materials.clone()
	}

	pub fn get_map(&mut self, map_name: &str) -> Option<SharedResourcePtr<BSPMap>>
	{
		if let Some(last_map) = &self.last_map
		{
			if last_map.0 == map_name
			{
				return Some(last_map.1.clone());
			}
		}

		let mut map_path = PathBuf::from(self.config.maps_path.clone());
		map_path.push(map_name);

		map_path = normalize_bsp_map_file_path(map_path);
		match load_map(&map_path)
		{
			Ok(Some(map)) =>
			{
				let map_rc = SharedResourcePtr::new(map);
				self.last_map = Some((map_name.to_string(), map_rc.clone()));
				Some(map_rc)
			},
			Ok(None) =>
			{
				self.console
					.lock()
					.unwrap()
					.add_text(format!("Failed to load map {:?}", map_path));
				None
			},
			Err(e) =>
			{
				self.console
					.lock()
					.unwrap()
					.add_text(format!("Failed to load map {:?}: {}", map_path, e));
				None
			},
		}
	}

	pub fn get_model(&mut self, key: &ResourceKey) -> SharedResourcePtr<triangle_model::TriangleModel>
	{
		if let Some(p) = self.models.get(key)
		{
			return p.clone();
		}

		let mut model_path = PathBuf::from(self.config.models_path.clone());
		model_path.push(key);

		// TODO - use dummy instead of "unwrap".
		let model = triangle_model_md3::load_model_md3(&model_path).unwrap().unwrap();

		let ptr = SharedResourcePtr::new(model);
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

		let ptr = SharedResourcePtr::new(image);
		self.images.insert(key.clone(), ptr.clone());

		ptr
	}

	pub fn get_material_texture(&mut self, key: &ResourceKey) -> SharedResourcePtr<TextureWithMips>
	{
		if let Some(p) = self.material_textures.get(key)
		{
			return p.clone();
		}

		let material = self.materials.get(key).unwrap_or_else(|| {
			self.console
				.lock()
				.unwrap()
				.add_text(format!("Failed to find material {:?}", key));
			&self.default_material
		});

		let texture_with_mips = load_texture(material, &self.config.textures_path);

		let ptr = SharedResourcePtr::new(texture_with_mips);
		self.material_textures.insert(key.clone(), ptr.clone());

		ptr
	}

	pub fn clear_cache(&mut self)
	{
		// Remove all resources that are stored only inside cache.
		remove_unused_resource_map_entries(&mut self.models);
		remove_unused_resource_map_entries(&mut self.images);
		remove_unused_resource_map_entries(&mut self.material_textures);
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
