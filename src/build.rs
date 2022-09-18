#[cfg(windows)]
use winres::WindowsResource;

fn main() -> std::io::Result<()>
{
	#[cfg(windows)]
	{
		WindowsResource::new().set_icon("../logo.ico").compile()?;
	}
	Ok(())
}
