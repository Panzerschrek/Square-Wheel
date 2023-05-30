use super::math_types::*;

#[derive(Debug)]
pub struct ParseError
{
	pub text_left: String,
}

impl ParseError
{
	pub fn build(it: Iterator) -> Self
	{
		ParseError {
			text_left: it[.. std::cmp::min(it.len(), 128)].to_string(),
		}
	}
}

pub type ParseResult<T> = Result<T, ParseError>;

pub type Iterator<'a> = &'a str;

pub fn parse_key_value_pair(it: &mut Iterator) -> ParseResult<(String, String)>
{
	let k = parse_quoted_string(it)?;
	skip_whitespaces(it);
	let v = parse_quoted_string(it)?;
	skip_whitespaces(it);

	Ok((k, v))
}

pub fn parse_whitespace_separated_string(it: &mut Iterator) -> ParseResult<String>
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

pub fn parse_quoted_string(it: &mut Iterator) -> ParseResult<String>
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

pub fn skip_whitespaces(it: &mut Iterator)
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

pub fn parse_vec3(s: Iterator) -> ParseResult<Vec3f>
{
	let mut it = s;
	Ok(Vec3f::new(
		parse_number(&mut it)?,
		parse_number(&mut it)?,
		parse_number(&mut it)?,
	))
}

pub fn parse_key_value_number(it: Iterator) -> ParseResult<f32>
{
	parse_number(&mut <&str>::clone(&it))
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

pub fn parse_brush_plane_vertex(it: &mut Iterator) -> ParseResult<Vec3f>
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
