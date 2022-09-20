use super::{frame_info::*, triangle_models_rendering::*};
use crate::common::{bbox::*, bsp_map_compact, math_types::*, matrix::*};
use std::sync::Arc;

// Class for placing dynamic objects (models, decals, lights, etc.) in bsp map.
// After placing it allows to query all BSP leafs where object is located and reverse - all objects inside given leaf.
pub struct DynamicObjectsIndex
{
	map: Arc<bsp_map_compact::BSPMap>,
	leafs_info: Vec<LeafInfo>,
	objects_info: Vec<ObjectInfo>,
}

// TODO - try to use some sort of vector with small internal storage to reduce heap allocations.

#[derive(Default, Clone)]
struct LeafInfo
{
	objects: Vec<DynamicObjectId>,
}

#[derive(Default, Clone)]
struct ObjectInfo
{
	leafs: Vec<u32>,
}

pub type DynamicObjectId = u32;

impl DynamicObjectsIndex
{
	pub fn new(map: Arc<bsp_map_compact::BSPMap>) -> Self
	{
		Self {
			leafs_info: vec![LeafInfo::default(); map.leafs.len()],
			objects_info: Vec::new(),
			map,
		}
	}

	pub fn get_leaf_objects(&self, leaf_index: u32) -> &[DynamicObjectId]
	{
		&self.leafs_info[leaf_index as usize].objects
	}

	pub fn get_object_leafs(&self, object_index: usize) -> &[u32]
	{
		&self.objects_info[object_index].leafs
	}

	// Reset internal state and position new set of models.
	pub fn position_models(&mut self, models: &[ModelEntity])
	{
		// Clear previous models.
		self.clear();

		// Position new models.
		self.allocate_objects(models.len());
		for (index, model) in models.iter().enumerate()
		{
			if model.is_view_model
			{
				// Do not place view models in BSP tree.
				continue;
			}

			self.position_object_bbox(
				index as DynamicObjectId,
				&get_current_triangle_model_bbox(&model.model, &model.animation),
				&get_object_matrix(model.position, model.rotation),
			);
		}
	}

	// Reset internal state and position new set of decals.
	pub fn position_decals(&mut self, decals: &[Decal])
	{
		// Clear previous decals.
		self.clear();

		// Position new decals.
		self.allocate_objects(decals.len());
		for (index, decal) in decals.iter().enumerate()
		{
			self.position_object_bbox(
				index as DynamicObjectId,
				&BBox::from_min_max(Vec3f::new(-1.0, -1.0, -1.0), Vec3f::new(1.0, 1.0, 1.0)),
				&get_object_matrix_with_scale(decal.position, decal.rotation, decal.scale),
			);
		}
	}

	// Reset internal state and position new set of dynamic lights.
	pub fn position_dynamic_lights(&mut self, lights: &[DynamicLight])
	{
		// Clear previous lights.
		self.clear();

		// Position new lights.
		self.allocate_objects(lights.len());
		for (index, light) in lights.iter().enumerate()
		{
			let root_node = bsp_map_compact::get_root_node_index(&self.map);
			if let DynamicLightShadowType::Projector { rotation, fov } = light.shadow_type
			{
				// Place projector lights using pyramid vertices.
				let matrix = get_object_matrix(light.position, rotation);
				let half_width = light.radius * (fov * 0.5).tan();

				let vertices_transformed = [
					Vec3f::new(0.0, 0.0, 0.0),
					Vec3f::new(light.radius, half_width, half_width),
					Vec3f::new(light.radius, half_width, -half_width),
					Vec3f::new(light.radius, -half_width, half_width),
					Vec3f::new(light.radius, -half_width, -half_width),
				]
				.map(|v| (matrix * v.extend(1.0)).truncate());

				self.position_object_convex_hull_r(index as DynamicObjectId, &vertices_transformed, root_node);
			}
			else
			{
				self.position_object_sphere_r(index as DynamicObjectId, &light.position, light.radius, root_node);
			}
		}
	}

	fn position_object_bbox(&mut self, id: DynamicObjectId, bbox: &BBox, transform_matrix: &Mat4f)
	{
		// transform bbox vertices.
		let bbox_vertices = bbox
			.get_corners_vertices()
			.map(|v| (transform_matrix * v.extend(1.0)).truncate());

		// Place bbox in leafs.
		let root_node = bsp_map_compact::get_root_node_index(&self.map);
		self.position_object_convex_hull_r(id, &bbox_vertices, root_node);
	}

	// Recursively place object in leafs. Perform convex hull vertices check against BPS node planes in order to do this.
	fn position_object_convex_hull_r<const NUM_VERTICES: usize>(
		&mut self,
		id: DynamicObjectId,
		vertices: &[Vec3f; NUM_VERTICES],
		node_index: u32,
	)
	{
		if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;
			self.leafs_info[leaf_index as usize].objects.push(id);
			self.objects_info[id as usize].leafs.push(leaf_index);
		}
		else
		{
			let node = &self.map.nodes[node_index as usize];

			let mut vertices_front = 0;
			for &vertex in vertices
			{
				if node.plane.vec.dot(vertex) > node.plane.dist
				{
					vertices_front += 1;
				}
			}

			let node_children = node.children;

			if vertices_front > 0
			{
				self.position_object_convex_hull_r(id, vertices, node_children[0]);
			}
			if vertices_front < vertices.len()
			{
				self.position_object_convex_hull_r(id, vertices, node_children[1]);
			}
		}
	}

	// Recursively place object in leafs. Perform sphere check against BPS node planes in order to do this.
	fn position_object_sphere_r(
		&mut self,
		id: DynamicObjectId,
		sphere_center: &Vec3f,
		sphere_radius: f32,
		node_index: u32,
	)
	{
		if node_index >= bsp_map_compact::FIRST_LEAF_INDEX
		{
			let leaf_index = node_index - bsp_map_compact::FIRST_LEAF_INDEX;
			self.leafs_info[leaf_index as usize].objects.push(id);
			self.objects_info[id as usize].leafs.push(leaf_index);
		}
		else
		{
			let node = &self.map.nodes[node_index as usize];
			let plane = node.plane;
			let node_children = node.children;

			// Scale sphere radius because plane vector may be unnormalized.
			let scaled_radius = sphere_radius * plane.vec.magnitude();
			let scaled_dist = sphere_center.dot(plane.vec);

			if scaled_dist + scaled_radius >= plane.dist
			{
				self.position_object_sphere_r(id, sphere_center, sphere_radius, node_children[0]);
			}
			if scaled_dist - scaled_radius <= plane.dist
			{
				self.position_object_sphere_r(id, sphere_center, sphere_radius, node_children[1]);
			}
		}
	}

	fn allocate_objects(&mut self, num_models: usize)
	{
		// Do not reize down to preserve internal allocations while resizing up again.
		if self.objects_info.len() < num_models
		{
			self.objects_info.resize(num_models, ObjectInfo::default());
		}
	}

	fn clear(&mut self)
	{
		for leafs_info in &mut self.leafs_info
		{
			leafs_info.objects.clear();
		}
		// Do not clear objects_info vector itself to preserve allocations in interlal vectors and avoid reallocations.
		for object_info in &mut self.objects_info
		{
			object_info.leafs.clear();
		}
	}
}
