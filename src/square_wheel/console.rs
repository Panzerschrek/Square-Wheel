use super::commands_queue;

pub struct Console
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	is_active: bool,
}

impl Console
{
	pub fn new() -> Self
	{
		Console {
			commands_queues: Vec::new(),
			is_active: false,
		}
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	pub fn toggle(&mut self)
	{
		self.is_active = !self.is_active;
	}

	pub fn is_active(&self) -> bool
	{
		self.is_active
	}
}
