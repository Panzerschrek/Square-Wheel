use super::{commands_queue, config};
use std::sync::{Arc, Mutex};

pub struct CommandsProcessor
{
	commands_queues: Vec<commands_queue::CommandsQueueDynPtr>,
	config: config::ConfigSharedPtr,
}

pub type CommandsProcessorPtr = Arc<Mutex<CommandsProcessor>>;

impl CommandsProcessor
{
	pub fn new(config: config::ConfigSharedPtr) -> CommandsProcessorPtr
	{
		Arc::new(Mutex::new(Self {
			commands_queues: Vec::new(),
			config,
		}))
	}

	pub fn register_command_queue(&mut self, queue: commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.push(queue);
	}

	pub fn remove_command_queue(&mut self, queue: &commands_queue::CommandsQueueDynPtr)
	{
		self.commands_queues.retain(|q| !Arc::ptr_eq(q, queue));
	}

	// Returns single string if successfully completed or list of variants.
	pub fn complete_command(&self, command_start: &str) -> Vec<String>
	{
		// Extract matched commands of all command queues.
		let mut matched_commands = Vec::new();
		for queue in &self.commands_queues
		{
			matched_commands.append(&mut queue.lock().unwrap().get_commands_started_with(command_start));
		}

		// Extract matched paths in config.
		let config_path = command_start.split(CONFIG_PATH_SEPARATOR).collect::<Vec<&str>>();
		if !config_path.is_empty()
		{
			let path_without_last_component = &config_path[.. config_path.len() - 1];
			let last_component = config_path.last().unwrap();

			let config_lock = self.config.lock().unwrap();
			let mut cur_value: &serde_json::Value = &config_lock;
			let mut path_found = true;
			for config_path_component in path_without_last_component
			{
				if let Some(member) = cur_value.get(config_path_component)
				{
					cur_value = member;
				}
				else
				{
					path_found = false;
					break;
				}
			}

			if path_found
			{
				if let Some(obj) = cur_value.as_object()
				{
					for key in obj.keys()
					{
						if key.starts_with(last_component)
						{
							let mut matched_path = path_without_last_component.join(CONFIG_PATH_SEPARATOR_STR);
							if !matched_path.is_empty()
							{
								matched_path.push(CONFIG_PATH_SEPARATOR);
							}
							matched_path += key;
							matched_commands.push(matched_path);
						}
					}
				}
			}
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

	// Returns command processing message, that may be empty.
	pub fn process_command(&mut self, command_line: &str) -> String
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

		let c = if let Some(c) = command
		{
			c
		}
		else
		{
			return String::new();
		};

		// First, process commands.
		for queue in &self.commands_queues
		{
			let mut queue_locked = queue.lock().unwrap();
			if queue_locked.has_handler(&c)
			{
				queue_locked.add_invocation(&c, args);
				return String::new();
			}
		}

		// Second, process config.
		let mut config_lock = self.config.lock().unwrap();
		let mut cur_value: &mut serde_json::Value = &mut config_lock;
		for config_path_component in c.split(CONFIG_PATH_SEPARATOR)
		{
			if let Some(member) = cur_value.get_mut(config_path_component)
			{
				cur_value = member;
			}
			else
			{
				return format!("{} not found", c);
			}
		}

		if args.is_empty()
		{
			return format!("{} is {}", c, cur_value);
		}

		let arg = &args[0];
		if cur_value.is_string()
		{
			*cur_value = serde_json::Value::from(arg.clone());
		}
		else if cur_value.is_u64()
		{
			if let Ok(num) = arg.parse::<u64>()
			{
				*cur_value = serde_json::Value::from(num);
			}
			else
			{
				return format!("Failed to parse u64");
			}
		}
		else if cur_value.is_i64()
		{
			if let Ok(num) = arg.parse::<i64>()
			{
				*cur_value = serde_json::Value::from(num);
			}
			else
			{
				return format!("Failed to parse i64");
			}
		}
		else if cur_value.is_f64()
		{
			if let Ok(num) = arg.parse::<f64>()
			{
				*cur_value = serde_json::Value::from(num);
			}
			else
			{
				return format!("Failed to parse f64");
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
				return format!("Failed to parse bool");
			}
		}
		else
		{
			return format!("Can't set value of this type");
		}

		String::new()
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

const CONFIG_PATH_SEPARATOR: char = '.';
const CONFIG_PATH_SEPARATOR_STR: &str = ".";
