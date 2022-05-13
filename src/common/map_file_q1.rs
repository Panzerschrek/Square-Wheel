use super::{map_file_common::*, math_types::*};

#[derive(Debug)]
pub struct BrushPlane
{
	pub vertices: [Vec3f; 3],
	pub texture: String,
	pub tc_offset: Vec2f,
	pub tc_scale: Vec2f,
	pub tc_angle: f32,
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

	let mut result = Brush::new();

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

	Ok(result)
}

fn parse_brush_plane(it: &mut Iterator) -> ParseResult<BrushPlane>
{
	Ok(BrushPlane {
		vertices: [
			parse_brush_plane_vertex(it)?,
			parse_brush_plane_vertex(it)?,
			parse_brush_plane_vertex(it)?,
		],
		texture: parse_whitespace_separated_string(it)?,
		tc_offset: Vec2f::new(parse_number(it)?, parse_number(it)?),
		tc_angle: parse_number(it)?,
		tc_scale: Vec2f::new(parse_number(it)?, parse_number(it)?),
	})
}
