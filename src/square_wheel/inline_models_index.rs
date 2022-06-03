use common::{bbox::*, bsp_map_compact, math_types::*};
use std::rc::Rc;

pub struct InlineModelsIndex
{
	map: Rc<bsp_map_compact::BSPMap>,
	leafs_info: Vec<LeafInfo>,
	models_info: Vec<ModelInfo>,
}

#[derive(Default, Clone)]
struct LeafInfo
{
	models: Vec<u32>,
}

#[derive(Clone)]
struct ModelInfo
{
	leafs: Vec<u32>,
	bbox: BBox,
	shift: Vec3f,
	// Rotation relative bbox center.
	// TODO - support arbitrary rotation.
	angle_z: RadiansF,
}

#[allow(dead_code)]
impl InlineModelsIndex
{
	pub fn new(map: Rc<bsp_map_compact::BSPMap>) -> Self
	{
		let mut result = Self {
			leafs_info: vec![LeafInfo::default(); map.leafs.len()],
			models_info: prepare_models_info(&map),
			map,
		};

		// Make initial positioning.
		for i in 0 .. result.models_info.len() as u32
		{
			result.force_reposition_model(i, &Vec3f::zero(), Rad(0.0));
		}

		result
	}

	pub fn reposition_model(&mut self, model_index: u32, shift: &Vec3f, angle_z: RadiansF)
	{
		let model_info = &self.models_info[model_index as usize];
		if model_info.shift != *shift || model_info.angle_z != angle_z
		{
			self.force_reposition_model(model_index, shift, angle_z);
		}
	}

	pub fn get_leaf_models(&self, leaf_index: u32) -> &[u32]
	{
		&self.leafs_info[leaf_index as usize].models
	}

	pub fn get_model_leafs(&self, model_index: u32) -> &[u32]
	{
		&self.models_info[model_index as usize].leafs
	}

	pub fn get_model_bbox(&self, model_index: u32) -> BBox
	{
		let model_info = &self.models_info[model_index as usize];
		if model_info.angle_z == Rad(0.0)
		{
			// No rotation - just return shifted bounding box.
			return BBox {
				min: model_info.bbox.min + model_info.shift,
				max: model_info.bbox.max + model_info.shift,
			};
		}

		// Transform original bounding box vertices and calculate new bounding box around these vertices.
		let bbox = &model_info.bbox;
		let transform_matrix = self.get_model_matrix(model_index);

		let mut bbox = BBox::from_point(&(transform_matrix * bbox.min.extend(1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.min.x, bbox.min.y, bbox.max.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.min.x, bbox.max.y, bbox.min.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.min.x, bbox.max.y, bbox.max.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.max.x, bbox.min.y, bbox.min.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.max.x, bbox.min.y, bbox.max.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.max.x, bbox.max.y, bbox.min.z, 1.0)).truncate());
		bbox.extend_with_point(&(transform_matrix * Vec4f::new(bbox.max.x, bbox.max.y, bbox.max.z, 1.0)).truncate());

		bbox
	}

	pub fn get_num_models(&self) -> usize
	{
		self.models_info.len()
	}

	pub fn get_model_matrix(&self, model_index: u32) -> Mat4f
	{
		let model_info = &self.models_info[model_index as usize];
		let center = model_info.bbox.get_center();
		Mat4f::from_translation(model_info.shift) *
			Mat4f::from_translation(center) *
			Mat4f::from_angle_z(model_info.angle_z) *
			Mat4f::from_translation(-center)
	}

	fn force_reposition_model(&mut self, model_index: u32, shift: &Vec3f, angle_z: RadiansF)
	{
		// First, erase this model index from models list of all leafs where this model was before.
		let model_info = &mut self.models_info[model_index as usize];
		for &leaf_index in &model_info.leafs
		{
			let leaf_info = &mut self.leafs_info[leaf_index as usize];
			leaf_info.models.retain(|index| *index != model_index);
		}
		// Reset model's leafs list.
		model_info.leafs.clear();

		// Set new position.
		model_info.shift = *shift;
		model_info.angle_z = angle_z;

		let bbox = model_info.bbox;
		let transform_matrix = self.get_model_matrix(model_index);

		// Calculate trasformed bounding box vertices.
		let bbox_vertices = [
			(transform_matrix * Vec4f::new(bbox.min.x, bbox.min.y, bbox.min.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.min.x, bbox.min.y, bbox.max.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.min.x, bbox.max.y, bbox.min.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.min.x, bbox.max.y, bbox.max.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.max.x, bbox.min.y, bbox.min.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.max.x, bbox.min.y, bbox.max.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.max.x, bbox.max.y, bbox.min.z, 1.0)).truncate(),
			(transform_matrix * Vec4f::new(bbox.max.x, bbox.max.y, bbox.max.z, 1.0)).truncate(),
		];

		// Place model in leafs.
		let root_node = (self.map.nodes.len() - 1) as u32;
		self.position_model_r(model_index, &bbox_vertices, root_node);
	}

	// Recursively place model in leafs. Perform bounding box vertices check agains BPS node planes in order to do this.
	fn position_model_r(&mut self, model_index: u32, bbox_vertices: &[Vec3f; 8], node_index: u32)
	{
		if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;
			self.leafs_info[leaf_index as usize].models.push(model_index);
			self.models_info[model_index as usize].leafs.push(leaf_index);
		}
		else
		{
			let node = &self.map.nodes[node_index as usize];

			let mut vertices_front = 0;
			for &vertex in bbox_vertices
			{
				if node.plane.vec.dot(vertex) > node.plane.dist
				{
					vertices_front += 1;
				}
			}

			let node_children = node.children;

			if vertices_front > 0
			{
				self.position_model_r(model_index, bbox_vertices, node_children[0]);
			}
			if vertices_front < bbox_vertices.len()
			{
				self.position_model_r(model_index, bbox_vertices, node_children[1]);
			}
		}
	}
}

fn prepare_models_info(map: &bsp_map_compact::BSPMap) -> Vec<ModelInfo>
{
	let mut result = Vec::with_capacity(map.submodels.len());

	for submodel in &map.submodels
	{
		result.push(prepare_model_info(map, submodel));
	}

	result
}

fn prepare_model_info(map: &bsp_map_compact::BSPMap, submodel: &bsp_map_compact::Submodel) -> ModelInfo
{
	ModelInfo {
		leafs: Vec::new(),
		bbox: bsp_map_compact::get_submodel_bbox(map, submodel),
		shift: Vec3f::zero(),
		angle_z: Rad(0.0),
	}
}
