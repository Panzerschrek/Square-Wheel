type Vec2f = cgmath::Vector2<f32>;
type Vec3f = cgmath::Vector3<f32>;

pub struct BrushPlane
{
	pub vertices: [Vec3f; 3],
	pub texture: String,
	pub tc_offset: Vec2f,
	pub tc_scale: Vec2f,
	pub tc_angle: f32,
}

pub type Brush = Vec<BrushPlane>;

#[derive(Default)]
pub struct Entity
{
	pub brushes: Vec<Brush>,
	pub keys: std::collections::HashMap<String, String>,
}

pub type MapFileParsed = Vec<Entity>;

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
		if it.starts_with("{")
		{
			result.push(parse_entity(&mut it)?);
		}
		else
		{
			return Err(ParseError::build(it));
		}
	}

	Ok(result)
}

fn parse_entity(it: &mut Iterator) -> ParseResult<Entity>
{
	*it = &it[1 ..]; // Skip "{"

	let mut result = Entity::default();

	while !it.is_empty() && !it.starts_with("}")
	{
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
	}

	if !it.starts_with("}")
	{
		return Err(ParseError::build(it));
	}

	Ok(result)
}

fn parse_brush(it: &mut Iterator) -> ParseResult<Brush>
{
	*it = &it[1 ..]; // Skip "{"

	let mut result = Brush::new();

	while !it.is_empty() && !it.starts_with("}")
	{
		// TODO
	}

	if !it.starts_with("}")
	{
		return Err(ParseError::build(it));
	}

	Ok(result)
}

fn parse_key_value_pair(it: &mut Iterator) -> ParseResult<(String, String)>
{
	Ok((parse_quoted_string(it)?, parse_quoted_string(it)?))
}

fn parse_quoted_string(it: &mut Iterator) -> ParseResult<String>
{
	// TODO
	Ok(String::new())
}
