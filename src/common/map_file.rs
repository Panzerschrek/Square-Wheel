use super::math_types::*;

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
pub struct ParseError
{
	pub text_left: String,
}

impl ParseError
{
	fn build(it: Iterator) -> Self
	{
		ParseError {
			text_left: it.to_string(),
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
		if it.starts_with("{")
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
	*it = &it[1 ..]; // Skip "{"

	let mut result = Entity::default();

	while !it.is_empty() && !it.starts_with("}")
	{
		skip_whitespaces(it);
		if it.starts_with("{")
		{
			result.brushes.push(parse_brush(it)?);
		}
		else if it.starts_with("\"")
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

	if !it.starts_with("}")
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..]; // Skip "}"

	Ok(result)
}

fn parse_brush(it: &mut Iterator) -> ParseResult<Brush>
{
	*it = &it[1 ..]; // Skip "{"

	let mut result = Brush::new();

	while !it.is_empty() && !it.starts_with("}")
	{
		result.push(parse_brush_plane(it)?);
		skip_whitespaces(it);
	}

	if !it.starts_with("}")
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..]; // Skip "}"

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
		tc_scale: Vec2f::new(parse_number(it)?, parse_number(it)?),
		tc_angle: parse_number(it)?,
	})
}

fn parse_brush_plane_vertex(it: &mut Iterator) -> ParseResult<Vec3f>
{
	skip_whitespaces(it);
	if !it.starts_with("(")
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	let result = Vec3f::new(parse_number(it)?, parse_number(it)?, parse_number(it)?);

	skip_whitespaces(it);
	if !it.starts_with(")")
	{
		return Err(ParseError::build(it));
	}
	*it = &it[1 ..];

	Ok(result)
}

fn parse_number(it: &mut Iterator) -> ParseResult<f32>
{
	let mut s = String::new();

	skip_whitespaces(it);
	while let Some(c) = it.chars().next()
	{
		if c == '.' || (c >= '0' && c <= '9') || c == '-'
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
		if c.is_ascii_whitespace()
		{
			*it = &it[1 ..];
			break;
		}
		else
		{
			*it = &it[1 ..];
			result.push(c);
		}
	}

	Ok(result)
}

fn parse_quoted_string(it: &mut Iterator) -> ParseResult<String>
{
	*it = &it[1 ..]; // Skip "

	let mut result = String::new();

	while !it.is_empty() && !it.starts_with("\"")
	{
		result.push_str(&it[0 .. 1]);
		*it = &it[1 ..];
	}

	if !it.starts_with("\"")
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
			while !it.is_empty() && !it.starts_with("\n")
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
