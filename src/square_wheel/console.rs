use super::commands_queue;

pub struct Console
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
}

impl Console
{
	pub fn new() -> Self
	{
		Console {
			commands_queues: Vec::new(),
		}
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}
}
