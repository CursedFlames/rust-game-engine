use std::collections::HashMap;
use std::env;
use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};

use anyhow::*;
use glob::glob;
use rayon::prelude::*;
use serde::Serialize;
use texture_packer::{Frame, TexturePacker, TexturePackerConfig};
use texture_packer::exporter::ImageExporter;
use texture_packer::importer::{ImageImporter, ImportResult};
use std::io::Write;

struct ImageData<Image> {
	pub name: String,
	pub image: Image,
}

// TODO pull this struct out into a common dependency instead of duplicating code between src and the buildscript
#[derive(Clone, Debug, Serialize)]
struct SpriteMetadata {
	/// x y w h of the (possibly trimmed) sprite
	pub frame: [u32; 4],
	// TODO actually use these?
	pub rotated: bool,
	// texture_packer seems to always set this to true?
	pub trimmed: bool,

	/// position of the trimmed frame within the original sprite
	pub original_position: [u32; 2],
	/// size of the sprite before trimming
	pub original_size: [u32; 2],
}

impl From<&Frame<String>> for SpriteMetadata {
	fn from(frame: &Frame<String>) -> Self {
		Self {
			frame: [frame.frame.x, frame.frame.y, frame.frame.w, frame.frame.h],
			rotated: frame.rotated,
			trimmed: frame.trimmed,
			original_position: [frame.source.x, frame.source.y],
			original_size: [frame.source.w, frame.source.h]
		}
	}
}

pub fn pack_textures() -> Result<()> {
	let out_dir_path: PathBuf = PathBuf::from(env::var("OUT_DIR")?).join("textures");

	let mut sprite_paths = Vec::new();
	sprite_paths.extend(glob("./assets/textures/sprites/**/*.png")?);
	let prefix = "assets/textures/sprites/";

	let mut sprites: Vec<ImageData<_>> = sprite_paths
		.into_par_iter()
		.map(|glob_result| {
			let glob = glob_result?;
			let image = ImageImporter::import_from_file(&glob).map_err(Error::msg)?;
			Ok(ImageData {
				image,
				// This is disgusting, but it works
				name: glob.strip_prefix(prefix)?
					.file_stem().ok_or(Error::msg("file_stem failed"))?
					.to_str().ok_or(Error::msg("to_str failed"))?
					.to_string()
			})
		})
		.collect::<Vec<Result<_>>>()
		.into_iter()
		.collect::<Result<Vec<_>>>()?;

	let mut texture_packer = TexturePacker::new_skyline(TexturePackerConfig {
		// TODO allow rotation?
		allow_rotation: false,
		trim: true,
		// TODO check if we're in a debug build or something?
		texture_outlines: false,
		..Default::default()
	});

	for sprite in sprites.into_iter() {
		texture_packer.pack_own(sprite.name, sprite.image);
	}

	let spritesheet = ImageExporter::export(&texture_packer).map_err(Error::msg)?;
	create_dir_all(&out_dir_path);
	// cargo_emit::warning!("{:?}", out_dir_path.join("spritesheet.png"));
	let mut file = File::create(out_dir_path.join("spritesheet.png")).map_err(Error::msg)?;
	spritesheet.write_to(&mut file, image::ImageFormat::Png).map_err(Error::msg)?;


	let frames = texture_packer.get_frames();
	let frames: HashMap<_, _> = frames.into_iter().map(|a| (a.0, SpriteMetadata::from(a.1))).collect();
	let frames_data = ron::to_string(&frames)?;
	let mut file = File::create(out_dir_path.join("spritesheet.ron")).map_err(Error::msg)?;
	file.write_all(frames_data.as_bytes());
	// loading will be like this:
	// let frames2 = ron::from_str::<HashMap<String, SpriteMetadata>(&string)?;

	Ok(())
}
