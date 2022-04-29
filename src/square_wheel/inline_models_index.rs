use common::{bsp_map_compact, math_types::*};
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
	bbox_min: Vec3f,
	bbox_max: Vec3f,
	shift: Vec3f,
}

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
			result.force_reposition_model(i, &Vec3f::zero());
		}

		result
	}

	pub fn reposition_model(&mut self, model_index: u32, shift: &Vec3f)
	{
		if self.models_info[model_index as usize].shift != *shift
		{
			self.force_reposition_model(model_index, shift);
		}
	}

	pub fn get_leaf_models(&self, leaf_index: u32) -> &[u32]
	{
		&self.leafs_info[leaf_index as usize].models
	}

	fn force_reposition_model(&mut self, model_index: u32, shift: &Vec3f)
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

		// Set new shift.
		model_info.shift = *shift;

		// Calculate new model.
		let bbox_vertices = [
			shift + Vec3f::new(model_info.bbox_min.x, model_info.bbox_min.y, model_info.bbox_min.z),
			shift + Vec3f::new(model_info.bbox_min.x, model_info.bbox_min.y, model_info.bbox_max.z),
			shift + Vec3f::new(model_info.bbox_min.x, model_info.bbox_max.y, model_info.bbox_min.z),
			shift + Vec3f::new(model_info.bbox_min.x, model_info.bbox_max.y, model_info.bbox_max.z),
			shift + Vec3f::new(model_info.bbox_max.x, model_info.bbox_min.y, model_info.bbox_min.z),
			shift + Vec3f::new(model_info.bbox_max.x, model_info.bbox_min.y, model_info.bbox_max.z),
			shift + Vec3f::new(model_info.bbox_max.x, model_info.bbox_max.y, model_info.bbox_min.z),
			shift + Vec3f::new(model_info.bbox_max.x, model_info.bbox_max.y, model_info.bbox_max.z),
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
				// TODO - check this.
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
	// Calculate model bounding box based on all vertices of all polygons.
	let inf = 1e24;
	let mut bbox_min = Vec3f::new(inf, inf, inf);
	let mut bbox_max = Vec3f::new(-inf, -inf, -inf);

	for &polygon in
		&map.polygons[(submodel.first_polygon as usize) .. ((submodel.first_polygon + submodel.num_polygons) as usize)]
	{
		for &vertex in
			&map.vertices[(polygon.first_vertex as usize) .. ((polygon.first_vertex + polygon.num_vertices) as usize)]
		{
			if vertex.x < bbox_min.x
			{
				bbox_min.x = vertex.x;
			}
			if vertex.x > bbox_max.x
			{
				bbox_max.x = vertex.x;
			}
			if vertex.y < bbox_min.y
			{
				bbox_min.y = vertex.y;
			}
			if vertex.y > bbox_max.y
			{
				bbox_max.y = vertex.y;
			}
			if vertex.z < bbox_min.z
			{
				bbox_min.z = vertex.z;
			}
			if vertex.z > bbox_max.z
			{
				bbox_max.z = vertex.z;
			}
		}
	}

	ModelInfo {
		leafs: Vec::new(),
		bbox_min,
		bbox_max,
		shift: Vec3f::zero(),
	}
}
