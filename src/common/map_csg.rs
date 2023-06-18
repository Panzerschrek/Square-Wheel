use super::{
	clipping_bsp::*,
	map_polygonizer::{Brush, MapPolygonized, Polygon},
	material,
};

#[derive(Debug, Clone)]
pub struct Entity
{
	pub polygons: Vec<Polygon>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapCSGProcessed = Vec<Entity>;

pub fn perform_no_csg_for_map_brushes(map: &MapPolygonized) -> MapCSGProcessed
{
	map.iter()
		.map(|e| {
			let mut polygons = Vec::new();
			for brush in &e.brushes
			{
				polygons.extend_from_slice(&brush);
			}
			Entity {
				polygons,
				keys: e.keys.clone(),
			}
		})
		.collect()
}

pub fn perform_csg_for_map_brushes(map: &MapPolygonized, materials: &material::MaterialsMap) -> MapCSGProcessed
{
	map.iter()
		.map(|e| Entity {
			polygons: perform_csg_for_entity_brushes(&e.brushes, materials),
			keys: e.keys.clone(),
		})
		.collect()
}

// CSG allows to remove polygons that are lying between two brushes, or polygons of one brushes, lying inside another brush.
// This prepass allows to reduce number of polygons and simplify further BSP building.
pub fn perform_csg_for_entity_brushes(brushes: &[Brush], materials: &material::MaterialsMap) -> Vec<Polygon>
{
	let mut result_polygons = Vec::new();

	// Fill table with solid flag for brushes.
	// Brush is solid if all faces are solid.
	let solid_flags = brushes
		.iter()
		.map(|brush| {
			for polygon in brush
			{
				if let Some(material) = materials.get(&polygon.texture_info.texture)
				{
					if !material.blocks_view
					{
						return false;
					}
				}
			}
			true
		})
		.collect::<Vec<_>>();

	for brush in brushes
	{
		if brush.is_empty()
		{
			continue;
		}

		let mut brush_polygons = brush.clone();

		// Set this flag to true for all brushes after this.
		// This is needed in order to choose only one polygons of two coplanar polygons of two intersecting brushes.
		let mut preserve_coplanar = false;

		for (other_brush, solid_flag) in brushes.iter().zip(solid_flags.iter())
		{
			// TODO - speed-up this, perform bbox check.
			if other_brush as *const Vec<Polygon> == brush as *const Vec<Polygon>
			{
				preserve_coplanar = true;
				continue;
			}

			if !solid_flag
			{
				continue;
			}

			let mut polygons_clipped = Vec::new();
			for polygon in brush_polygons.drain(..)
			{
				polygons_clipped.append(&mut cut_polygon_by_brush_planes(
					polygon,
					other_brush,
					preserve_coplanar,
				));
			}
			brush_polygons = polygons_clipped;
		}

		result_polygons.append(&mut brush_polygons);
	}

	result_polygons
}

fn cut_polygon_by_brush_planes(polygon: Polygon, brush: &Vec<Polygon>, preserve_coplanar: bool) -> Vec<Polygon>
{
	// Check if this polygon is trivially outisde.
	for brush_polygon in brush
	{
		if get_polygon_position_relative_plane(&polygon, &brush_polygon.plane) == PolygonPositionRelativePlane::Front
		{
			return vec![polygon];
		}
	}

	// Cut polygon into pieces by sides of this brush.
	let mut result_polygons = Vec::new();
	let mut leftover_polygon = polygon.clone();

	for brush_polygon in brush
	{
		match get_polygon_position_relative_plane(&leftover_polygon, &brush_polygon.plane)
		{
			PolygonPositionRelativePlane::Front =>
			{
				// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
				return vec![polygon];
			},
			PolygonPositionRelativePlane::Back =>
			{
				// Leftover polygon is possible inside the brush - continue splitting.
			},
			PolygonPositionRelativePlane::CoplanarFront =>
			{
				// We need to save polygon only if same polygon of other brush was previously skipped.
				if preserve_coplanar
				{
					// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
					return vec![polygon];
				}
			},
			PolygonPositionRelativePlane::CoplanarBack =>
			{
				// Preserve coplanar leftover polygon.
			},
			PolygonPositionRelativePlane::Splitted =>
			{
				let (front_polygon, back_polygon) = split_polygon(&leftover_polygon, &brush_polygon.plane);
				if front_polygon.vertices.len() >= 3
				{
					result_polygons.push(front_polygon); // Front polygon piece is outside brush - preserve it.
				}

				if back_polygon.vertices.len() >= 3
				{
					// Continue clipping of inside piese.
					leftover_polygon = back_polygon;
				}
				else
				{
					// Leftover polygon is outside the brush - splitting was unnecessary - return initial polygon.
					return vec![polygon];
				}
			},
		};
	} // for brush planes.

	result_polygons
}
