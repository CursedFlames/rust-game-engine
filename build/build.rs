mod shaders;

use anyhow::*;
use crate::shaders::compile_shaders;

fn main() -> Result<()> {
	compile_shaders()?;
	Ok(())
}
