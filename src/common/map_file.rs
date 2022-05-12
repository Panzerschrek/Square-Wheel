use super::{math_types::*, plane::*};

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

#[derive(Debug)]
pub struct BrushPlaneQ4
{
	pub plane: Plane,
	pub tex_axis: [TexAxisQ4; 2],
	pub texture: String,
}

#[derive(Debug)]
pub struct TexAxisQ4
{
	pub scale: Vec2f,
	pub offset: f32,
}

pub type BrushQ4 = Vec<BrushPlaneQ4>;

#[derive(Default, Debug)]
pub struct EntityQ4
{
	pub brushes: Vec<BrushQ4>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapFileParsedQ4 = Vec<EntityQ4>;

#[derive(Debug)]
pub struct ParseError
{
	pub text_left: String,
}

impl ParseError
{
	fn build(it: Iterator) -> Self
	{
		ParseError {
			text_left: it[.. std::cmp::min(it.len(), 128)].to_string(),
		}
	}
}

type ParseResult<T> = Result<T, ParseError>;

type Iterator<'a> = &'a str;

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

pub fn parse_map_file_content_q4(content: Iterator) -> ParseResult<MapFileParsedQ4>
{
	let mut result = MapFileParsedQ4::new();

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
			result.push(parse_entity_q4(&mut it)?);
		}
		else
		{
			return Err(ParseError::build(it));
		}
		skip_whitespaces(&mut it);
	}

	Ok(result)
}

fn parse_entity_q4(it: &mut Iterator) -> ParseResult<EntityQ4>
{
	*it = &it[1 ..]; // Skip '{'

	let mut result = EntityQ4::default();

	while !it.is_empty() && !it.starts_with('}')
	{
		skip_whitespaces(it);
		if it.starts_with('{')
		{
			result.brushes.push(parse_brush_q4(it)?);
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

fn parse_brush_q4(it: &mut Iterator) -> ParseResult<BrushQ4>
{
	*it = &it[1 ..]; // Skip '{'
	skip_whitespaces(it);

	let mut result = BrushQ4::new();

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
			result.push(parse_brush_plane_q4(it)?);
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

fn parse_brush_plane_q4(it: &mut Iterator) -> ParseResult<BrushPlaneQ4>
{
	let plane = parse_plane_equation(it)?;
	skip_whitespaces(it);
	let tex_axis = parse_tex_axis_q4(it)?;
	skip_whitespaces(it);
	let texture = parse_quoted_string(it)?;

	Ok(BrushPlaneQ4 {
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

fn parse_tex_axis_q4(it: &mut Iterator) -> ParseResult<[TexAxisQ4; 2]>
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
		TexAxisQ4 {
			scale: v0.truncate(),
			offset: v0.z,
		},
		TexAxisQ4 {
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

pub fn parse_vec3(s: Iterator) -> ParseResult<Vec3f>
{
	let mut it = s;
	Ok(Vec3f::new(
		parse_number(&mut it)?,
		parse_number(&mut it)?,
		parse_number(&mut it)?,
	))
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

fn parse_brush_plane_vertex(it: &mut Iterator) -> ParseResult<Vec3f>
{
	skip_whitespaces(it);
	if !it.starts_with('(')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	let result = Vec3f::new(parse_number(it)?, parse_number(it)?, parse_number(it)?);

	skip_whitespaces(it);
	if !it.starts_with(')')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	Ok(result)
}

pub fn parse_number(it: &mut Iterator) -> ParseResult<f32>
{
	let mut s = String::new();

	skip_whitespaces(it);
	while let Some(c) = it.chars().next()
	{
		if c == '.' || ('0' ..= '9').contains(&c) || c == '-'
		{
			s.push(c);
			*it = &it[1 ..];
		}
		else
		{
			break;
		}
	}

	if let Ok(parse_res) = s.parse::<f32>()
	{
		Ok(parse_res)
	}
	else
	{
		Err(ParseError::build(it))
	}
}

fn parse_key_value_pair(it: &mut Iterator) -> ParseResult<(String, String)>
{
	let k = parse_quoted_string(it)?;
	skip_whitespaces(it);
	let v = parse_quoted_string(it)?;
	skip_whitespaces(it);

	Ok((k, v))
}

fn parse_whitespace_separated_string(it: &mut Iterator) -> ParseResult<String>
{
	let mut result = String::new();

	skip_whitespaces(it);
	while let Some(c) = it.chars().next()
	{
		*it = &it[1 ..];
		if c.is_ascii_whitespace()
		{
			break;
		}
		else
		{
			result.push(c);
		}
	}

	Ok(result)
}

fn parse_quoted_string(it: &mut Iterator) -> ParseResult<String>
{
	*it = &it[1 ..]; // Skip "

	let mut result = String::new();

	while !it.is_empty() && !it.starts_with('"')
	{
		result.push_str(&it[0 .. 1]);
		*it = &it[1 ..];
	}

	if !it.starts_with('"')
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..]; // Skip "

	Ok(result)
}

fn skip_whitespaces(it: &mut Iterator)
{
	while let Some(c) = it.chars().next()
	{
		if c.is_ascii_whitespace()
		{
			*it = &it[1 ..];
		}
		else if it.starts_with("//")
		{
			// Comments
			*it = &it[2 ..];
			while !it.is_empty() && !it.starts_with('\n')
			{
				*it = &it[1 ..];
			}
		}
		else
		{
			break;
		}
	}
}
