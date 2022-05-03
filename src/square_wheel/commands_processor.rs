use super::{commands_queue, config};
use std::{cell::RefCell, rc::Rc};

pub struct CommandsProcessor
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	config: config::ConfigSharedPtr,
}

pub type CommandsProcessorPtr = Rc<RefCell<CommandsProcessor>>;

impl CommandsProcessor
{
	pub fn new(config: config::ConfigSharedPtr) -> CommandsProcessorPtr
	{
		Rc::new(RefCell::new(Self {
			commands_queues: Vec::new(),
			config,
		}))
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	// Returns single string if successfully completed or list of variants.
	pub fn complete_command(&self, command_start: &str) -> Vec<String>
	{
		let mut matched_commands = Vec::new();
		for queue in &self.commands_queues
		{
			matched_commands.append(&mut queue.borrow().get_commands_started_with(command_start));
		}

		if matched_commands.len() <= 1
		{
			return matched_commands;
		}

		let common_prefix = find_common_prefix(&matched_commands);

		// Return common prefix if it is longer, than initial command start.
		// Else return sorted list of possible commands.
		if common_prefix.len() > command_start.len()
		{
			vec![common_prefix]
		}
		else
		{
			matched_commands.sort();
			matched_commands
		}
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

			// TODO - print messages to console.

			let mut config_lock = self.config.borrow_mut();
			let mut cur_value: &mut serde_json::Value = &mut config_lock;
			for config_path_component in c.split('.')
			{
				if let Some(member) = cur_value.get_mut(config_path_component)
				{
					cur_value = member;
				}
				else
				{
					println!("{} not found", c);
					return false;
				}
			}

			if args.is_empty()
			{
				println!("{} is {}", c, cur_value);
			}
			else
			{
				let arg = &args[0];
				if cur_value.is_string()
				{
					*cur_value = serde_json::Value::from(arg.clone());
				}
				else if cur_value.is_number()
				{
					if let Ok(num) = arg.parse::<f64>()
					{
						*cur_value = serde_json::Value::from(num);
					}
					else
					{
						println!("Failed to parse number");
					}
				}
				else if cur_value.is_boolean()
				{
					if arg == "1" || arg == "true"
					{
						*cur_value = serde_json::Value::from(true);
					}
					else if arg == "0" || arg == "false"
					{
						*cur_value = serde_json::Value::from(false);
					}
					else
					{
						println!("Failed to parse bool");
					}
				}
				else
				{
					println!("Can't set value of this type");
				}
			}

			return true;
		}

		false
	}
}

fn find_common_prefix(strings: &[String]) -> String
{
	if strings.is_empty()
	{
		return String::new();
	}

	let mut iters = Vec::new();
	for s in strings
	{
		iters.push(s.chars());
	}

	let mut common_prefix = String::new();
	loop
	{
		let mut all_eq = true;
		let mut current_char = None;
		for iter in &mut iters
		{
			if let Some(c) = iter.next()
			{
				if let Some(prev_c) = current_char
				{
					if prev_c != c
					{
						all_eq = false;
						break;
					}
				}
				else
				{
					current_char = Some(c);
				}
			}
			else
			{
				all_eq = false;
				break;
			}
		}

		if all_eq
		{
			common_prefix.push(current_char.unwrap());
		}
		else
		{
			break;
		}
	}

	common_prefix
}
