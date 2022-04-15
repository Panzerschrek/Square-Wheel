use super::commands_queue;
use std::{cell::RefCell, rc::Rc};

pub struct CommandsProcessor
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
}

pub type CommandsProcessorPtr = Rc<RefCell<CommandsProcessor>>;

impl CommandsProcessor
{
	pub fn new() -> CommandsProcessorPtr
	{
		Rc::new(RefCell::new(Self {
			commands_queues: Vec::new(),
		}))
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	// Returns true if command found.
	pub fn process_command(&mut self, command_line: &str) -> bool
	{
		let mut command = None;
		let mut args = Vec::<String>::new();
		for token in command_line.split_ascii_whitespace()
		{
			if command.is_none()
			{
				command = Some(token.to_string());
			}
			else
			{
				args.push(token.to_string());
			}
		}

		if let Some(c) = command
		{
			for queue in &self.commands_queues
			{
				if queue.borrow().has_handler(&c)
				{
					queue.borrow_mut().add_invocation(&c, args);
					return true;
				}
			}
		}

		false
	}
}
