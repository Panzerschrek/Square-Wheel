// Class for average (window-based) calculation of some performance metric.

pub struct PerformanceCounter
{
	cur_pos: usize,
	values: Vec<f32>,
}

impl PerformanceCounter
{
	pub fn new(window_size: usize) -> Self
	{
		PerformanceCounter {
			cur_pos: 0,
			values: vec![0.0; window_size],
		}
	}

	pub fn add_value(&mut self, value: f32)
	{
		self.values[self.cur_pos] = value;
		self.cur_pos += 1;
		if self.cur_pos >= self.values.len()
		{
			self.cur_pos = 0;
		}
	}

	pub fn get_average_value(&self) -> f32
	{
		let mut sum = 0.0;
		for value in &self.values
		{
			sum += value
		}
		sum / (self.values.len() as f32)
	}

	pub fn run_with_measure<F: FnOnce()>(&mut self, f: F)
	{
		type Clock = std::time::Instant;
		let start_time = Clock::now();

		f();

		let end_time = Clock::now();
		self.add_value((end_time - start_time).as_secs_f32());
	}
}
