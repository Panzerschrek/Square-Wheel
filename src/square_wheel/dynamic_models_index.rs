use super::{frame_info::*, triangle_models_rendering::*};
use common::{bsp_map_compact, math_types::*, matrix::*};
use std::sync::Arc;

pub struct DynamicModelsIndex
{
	map: Arc<bsp_map_compact::BSPMap>,
	leafs_info: Vec<LeafInfo>,
	models_info: Vec<ModelInfo>,
}

#[derive(Default, Clone)]
struct LeafInfo
{
	models: Vec<ModelId>,
}

#[derive(Default, Clone)]
struct ModelInfo
{
	leafs: Vec<u32>,
}

pub type ModelId = u32;

impl DynamicModelsIndex
{
	pub fn new(map: Arc<bsp_map_compact::BSPMap>) -> Self
	{
		Self {
			leafs_info: vec![LeafInfo::default(); map.leafs.len()],
			models_info: Vec::new(),
			map,
		}
	}

	pub fn get_leaf_models(&self, leaf_index: u32) -> &[u32]
	{
		&self.leafs_info[leaf_index as usize].models
	}

	pub fn get_model_leafs(&self, model_index: usize) -> &[u32]
	{
		&self.models_info[model_index].leafs
	}

	// Reet internal state and position new set of models.
	pub fn position_models(&mut self, models: &[ModelEntity])
	{
		// Clear previous models.
		self.clear();

		// Position new models.
		self.models_info.resize(models.len(), ModelInfo::default());
		for (index, model) in models.iter().enumerate()
		{
			if model.is_view_model
			{
				// Do not place view models in BSP tree.
				continue;
			}
			self.position_model(model, index as ModelId);
		}
	}

	fn position_model(&mut self, model: &ModelEntity, id: ModelId)
	{
		// Calculate current bounding box.
		let bbox = get_current_triangle_model_bbox(&model.model, &model.animation);
		let transform_matrix = get_object_matrix(model.position, model.angles);

		let bbox_vertices = bbox
			.get_corners_vertices()
			.map(|v| (transform_matrix * v.extend(1.0)).truncate());

		// Place model in leafs.
		let root_node = (self.map.nodes.len() - 1) as u32;
		self.position_model_r(id, &bbox_vertices, root_node);
	}

	// Recursively place model in leafs. Perform bounding box vertices check agains BPS node planes in order to do this.
	fn position_model_r(&mut self, id: ModelId, bbox_vertices: &[Vec3f; 8], node_index: u32)
	{
		if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;
			self.leafs_info[leaf_index as usize].models.push(id);
			self.models_info[id as usize].leafs.push(leaf_index);
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
				self.position_model_r(id, bbox_vertices, node_children[0]);
			}
			if vertices_front < bbox_vertices.len()
			{
				self.position_model_r(id, bbox_vertices, node_children[1]);
			}
		}
	}

	fn clear(&mut self)
	{
		for leafs_info in &mut self.leafs_info
		{
			leafs_info.models.clear();
		}
	}
}
