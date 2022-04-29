use common::bsp_map_compact;
use std::rc::Rc;

pub struct InlineModelsIndex
{
	map: Rc<bsp_map_compact::BSPMap>,
}

impl InlineModelsIndex
{
	pub fn new(map: Rc<bsp_map_compact::BSPMap>) -> Self
	{
		Self { map }
	}
}
