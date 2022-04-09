pub struct TicksCounter
{
	update_interval_s: f32,
	total_ticks: u32,
	current_sample_ticks: u32,
	output_ticks_frequency: f32,
	last_update_time: std::time::Instant,
}

impl TicksCounter
{
	pub fn new() -> Self
	{
		TicksCounter {
			update_interval_s: 1.0,
			total_ticks: 0,
			current_sample_ticks: 0,
			output_ticks_frequency: 0.0,
			last_update_time: std::time::Instant::now(),
		}
	}

	pub fn tick(&mut self)
	{
		self.total_ticks += 1;
		self.current_sample_ticks += 1;

		let current_time = std::time::Instant::now();
		let time_since_last_update_s = (current_time - self.last_update_time).as_secs_f32();
		if time_since_last_update_s >= self.update_interval_s
		{
			self.output_ticks_frequency = (self.current_sample_ticks as f32) / time_since_last_update_s;
			self.last_update_time = current_time;
			self.current_sample_ticks = 0;
		}
	}

	pub fn get_frequency(&self) -> f32
	{
		self.output_ticks_frequency
	}
}
