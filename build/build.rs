use anyhow::*;
use rayon::join;
use tuple_transpose::TupleTranspose;

use crate::shaders::compile_shaders;
use crate::textures::pack_textures;

mod shaders;
mod textures;

fn main() -> Result<()> {
	join(
		compile_shaders,
		pack_textures
	).transpose()?;
	Ok(())
}
