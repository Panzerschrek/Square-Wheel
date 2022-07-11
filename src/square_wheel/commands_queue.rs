use std::sync::{Arc, Mutex};

pub type CommandArgs = Vec<String>;
pub type CommandHandler<HandlerClass> = fn(&mut HandlerClass, CommandArgs);
pub type NamedCommandHandler<HandlerClass> = (&'static str, CommandHandler<HandlerClass>);

pub trait CommandsQueueInterface: Send + Sync
{
	fn has_handler(&self, command: &str) -> bool;
	fn add_invocation(&mut self, command: &str, args: CommandArgs);
	fn get_commands_started_with(&self, command_start: &str) -> Vec<String>;
}

pub struct CommandsQueue<HandlerClass>
{
	handlers: Vec<NamedCommandHandler<HandlerClass>>,

	// Queue of invocations for each registered command.
	invocations: Vec<Vec<CommandArgs>>,
}

pub type CommandsQueuePtr<HandlerClass> = Arc<Mutex<CommandsQueue<HandlerClass>>>;
pub type CommandsQueueDynPtr = Arc<Mutex<dyn CommandsQueueInterface>>;

impl<HandlerClass> CommandsQueue<HandlerClass>
{
	pub fn new(handlers: Vec<NamedCommandHandler<HandlerClass>>) -> CommandsQueuePtr<HandlerClass>
	{
		let invocations = vec![Vec::new(); handlers.len()];
		Arc::new(Mutex::new(Self { handlers, invocations }))
	}

	pub fn process_commands(&mut self, handler: &mut HandlerClass)
	{
		for ((_, func), invocations) in self.handlers.iter().zip(self.invocations.iter_mut())
		{
			for invocation in invocations.drain(..)
			{
				func(handler, invocation);
			}
		}
	}
}

impl<HandlerClass> CommandsQueueInterface for CommandsQueue<HandlerClass>
{
	fn has_handler(&self, command: &str) -> bool
	{
		for (command_name, _) in &self.handlers
		{
			if command == *command_name
			{
				return true;
			}
		}

		false
	}

	fn add_invocation(&mut self, command: &str, args: CommandArgs)
	{
		for ((command_name, _), invocations) in self.handlers.iter().zip(self.invocations.iter_mut())
		{
			if command == *command_name
			{
				invocations.push(args);
				return;
			}
		}
	}

	fn get_commands_started_with(&self, command_start: &str) -> Vec<String>
	{
		let mut result = Vec::new();
		for (command_name, _) in &self.handlers
		{
			if command_name.starts_with(command_start) || command_start.is_empty()
			{
				result.push(command_name.to_string());
			}
		}
		result
	}
}
