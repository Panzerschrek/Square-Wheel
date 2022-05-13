use super::{map_file_common::*, math_types::*, plane::*};

#[derive(Debug)]
pub struct BrushPlane
{
	pub plane: Plane,
	pub tex_axis: [TexAxis; 2],
	pub texture: String,
}

#[derive(Debug)]
pub struct TexAxis
{
	pub scale: Vec2f,
	pub offset: f32,
}

pub type Brush = Vec<BrushPlane>;

#[derive(Default, Debug)]
pub struct Entity
{
	pub brushes: Vec<Brush>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapFileParsed = Vec<Entity>;

pub fn parse_map_file_content(content: Iterator) -> ParseResult<MapFileParsed>
{
	let mut result = MapFileParsed::new();

	let mut it: Iterator = content;

	let version_keyword = "Version";
	let expected_version = "3";

	if !it.starts_with(version_keyword)
	{
		return Err(ParseError::build(it));
	}
	it = &it[version_keyword.len() ..];

	skip_whitespaces(&mut it);

	if !it.starts_with(expected_version)
	{
		return Err(ParseError::build(it));
	}
	it = &it[expected_version.len() ..];

	while !it.is_empty()
	{
		skip_whitespaces(&mut it);
		if it.starts_with('{')
		{
			result.push(parse_entity(&mut it)?);
		}
		else
		{
			return Err(ParseError::build(it));
		}
		skip_whitespaces(&mut it);
	}

	Ok(result)
}

fn parse_entity(it: &mut Iterator) -> ParseResult<Entity>
{
	*it = &it[1 ..]; // Skip '{'

	let mut result = Entity::default();

	while !it.is_empty() && !it.starts_with('}')
	{
		skip_whitespaces(it);
		if it.starts_with('{')
		{
			result.brushes.push(parse_brush(it)?);
		}
		else if it.starts_with('"')
		{
			let kv = parse_key_value_pair(it)?;
			result.keys.insert(kv.0, kv.1);
		}
		else
		{
			return Err(ParseError::build(it));
		}
		skip_whitespaces(it);
	}

	if !it.starts_with('}')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..]; // Skip "}"

	Ok(result)
}

fn parse_brush(it: &mut Iterator) -> ParseResult<Brush>
{
	*it = &it[1 ..]; // Skip '{'
	skip_whitespaces(it);

	let mut result = Brush::new();

	let brush_def = "brushDef3";
	let patch_def2 = "patchDef2";
	let patch_def3 = "patchDef3";

	if it.starts_with(brush_def)
	{
		*it = &it[brush_def.len() ..];
		skip_whitespaces(it);

		if !it.starts_with('{')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..]; // Skip '{'

		while !it.is_empty() && !it.starts_with('}')
		{
			result.push(parse_brush_plane(it)?);
			skip_whitespaces(it);
		}

		if !it.starts_with('}')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..]; // Skip '}'
	}
	else if it.starts_with(patch_def2) || it.starts_with(patch_def3)
	{
		*it = &it[patch_def2.len() ..];
		skip_whitespaces(it);

		if !it.starts_with('{')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..]; // Skip '{'
		skip_whitespaces(it);

		parse_patch_def_body(it)?;

		skip_whitespaces(it);
		if !it.starts_with('}')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..]; // Skip '}'
	}
	else
	{
		return Err(ParseError::build(it));
	}

	skip_whitespaces(it);
	if !it.starts_with('}')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..]; // Skip '}'

	Ok(result)
}

fn parse_brush_plane(it: &mut Iterator) -> ParseResult<BrushPlane>
{
	let plane = parse_plane_equation(it)?;
	skip_whitespaces(it);
	let tex_axis = parse_tex_axis(it)?;
	skip_whitespaces(it);
	let texture = parse_quoted_string(it)?;

	Ok(BrushPlane {
		plane,
		texture,
		tex_axis,
	})
}

fn parse_plane_equation(it: &mut Iterator) -> ParseResult<Plane>
{
	skip_whitespaces(it);
	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	let result = Plane {
		vec: Vec3f::new(parse_number(it)?, parse_number(it)?, parse_number(it)?),
		dist: -parse_number(it)?,
	};

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	Ok(result)
}

fn parse_tex_axis(it: &mut Iterator) -> ParseResult<[TexAxis; 2]>
{
	skip_whitespaces(it);
	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	let v0 = parse_brush_plane_vertex(it)?;
	let v1 = parse_brush_plane_vertex(it)?;

	let result = [
		TexAxis {
			scale: v0.truncate(),
			offset: v0.z,
		},
		TexAxis {
			scale: v1.truncate(),
			offset: v1.z,
		},
	];

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	Ok(result)
}

fn parse_patch_def_body(it: &mut Iterator) -> ParseResult<()>
{
	let _texture = parse_quoted_string(it)?;
	parse_patch_header(it)?;

	skip_whitespaces(it);

	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	while !it.is_empty() && !it.starts_with(')')
	{
		skip_whitespaces(it);
		if !it.starts_with('(')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..];
		skip_whitespaces(it);

		for _i in 0 .. 3
		{
			skip_whitespaces(it);
			parse_patch_control_point(it)?;
		}

		skip_whitespaces(it);
		if !it.starts_with(')')
		{
			return Err(ParseError::build(it));
		}
		*it = &it[1 ..];
		skip_whitespaces(it);
	}

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	Ok(())
}

fn parse_patch_header(it: &mut Iterator) -> ParseResult<Vec<f32>>
{
	skip_whitespaces(it);
	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	let mut result = Vec::new();
	while !it.is_empty() && !it.starts_with(')')
	{
		result.push(parse_number(it)?);
		skip_whitespaces(it);
	}

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	Ok(result)
}

fn parse_patch_control_point(it: &mut Iterator) -> ParseResult<[f32; 5]>
{
	skip_whitespaces(it);
	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	let result = [
		parse_number(it)?,
		parse_number(it)?,
		parse_number(it)?,
		parse_number(it)?,
		parse_number(it)?,
	];

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];
	skip_whitespaces(it);

	Ok(result)
}
