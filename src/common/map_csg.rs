use super::{
	clipping_bsp::*,
	map_polygonizer::{MapPolygonized, Polygon},
};

#[derive(Debug, Clone)]
pub struct Entity
{
	pub polygons: Vec<Polygon>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapCSGProcessed = Vec<Entity>;

pub fn perform_csg_for_map_brushes(map: &MapPolygonized) -> MapCSGProcessed
{
	map.iter()
		.map(|e| Entity {
			polygons: perform_csg_for_entity_brushes(&e.brushes),
			keys: e.keys.clone(),
		})
		.collect()
}

pub fn perform_csg_for_entity_brushes(brushes: &[Vec<Polygon>]) -> Vec<Polygon>
{
	let mut result_polygons = Vec::new();

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

		for other_brush in brushes
		{
			// TODO - speed-up this, perform bbox check.
			if other_brush as *const Vec<Polygon> == brush as *const Vec<Polygon>
			{
				preserve_coplanar = true;
				continue;
			}

			// TODO - ignore brushes with non-blocking textures and/or semitransparent textures.

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

fn cut_polygon_by_brush_planes(mut polygon: Polygon, brush: &Vec<Polygon>, preserve_coplanar: bool) -> Vec<Polygon>
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
	// TODO - this can still perform unnecessary splits for polygons, that are totally outside. Handle such cases.

	let mut result_polygons = Vec::new();

	for brush_polygon in brush
	{
		match get_polygon_position_relative_plane(&polygon, &brush_polygon.plane)
		{
			PolygonPositionRelativePlane::Front =>
			{
				// Leftover polygon is outside the brush - can stop splitting.
				result_polygons.push(polygon);
				break;
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
					result_polygons.push(polygon);
					break;
				}
			},
			PolygonPositionRelativePlane::CoplanarBack =>
			{
				// Preserve coplanar leftover polygon.
			},
			PolygonPositionRelativePlane::Splitted =>
			{
				let (front_polygon, back_polygon) = split_polygon(&polygon, &brush_polygon.plane);
				if front_polygon.vertices.len() >= 3
				{
					result_polygons.push(front_polygon); // Front polygon piece is outside brush - preserve it.
				}

				if back_polygon.vertices.len() >= 3
				{
					// Continue clipping of inside piese.
					polygon = back_polygon;
				}
				else
				{
					// Nothing left inside.
					break;
				}
			},
		};
	} // for brush planes.

	result_polygons
}
