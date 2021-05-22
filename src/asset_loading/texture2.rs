use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::{File, read_to_string};
use std::io::Read;
use std::num::NonZeroU32;
use std::path::Path;

use anyhow::*;
use image::{ColorType, GenericImageView, ImageFormat};
use log::*;
use wgpu::*;

use crate::render::sprite::{SpriteMetadata, Spritesheet};

pub struct TextureData {
	pub bytes: Vec<u8>,
	pub dimensions: Extent3d,
	pub format: TextureFormat,
}

fn color_type_matches_format(color: ColorType, format: TextureFormat) -> bool {
	use ColorType::*;
	use TextureFormat::*;
	match color {
		// Always fail on 3-channel formats - we convert them to 4-channel before calling this method anyway
		Bgr8 | Rgb8 | Rgb16 => false,
		// Bgra8 is special since it has reversed order
		Bgra8 => match format { Bgra8Unorm | Bgra8UnormSrgb => true, _ => false },
		// Normal formats
		L8 => match format { R8Unorm | R8Snorm | R8Uint | R8Sint => true, _ => false },
		La8 => match format { Rg8Unorm | Rg8Snorm | Rg8Uint | Rg8Sint => true, _ => false },
		Rgba8 => match format { Rgba8Unorm | Rgba8UnormSrgb | Rgba8Snorm | Rgba8Uint | Rgba8Sint => true, _ => false },
		L16 => match format { R16Uint | R16Sint | R16Float => true, _ => false },
		La16 => match format { Rg16Uint | Rg16Sint | Rg16Float => true, _ => false },
		Rgba16 => match format { Rgba16Uint | Rgba16Sint | Rgba16Float => true, _ => false },
		_ => { warn!("Unrecognized color type {:?}", color); false }
	}
}

pub fn load_image_from_file<P: AsRef<Path>>(path: P, expected_format: Option<ImageFormat>, texture_format: TextureFormat) -> Result<TextureData> {
	let mut file = File::open(path)?;
	let mut data = Vec::new();
	file.read_to_end(&mut data)?;
	load_image_data(data.as_ref(), expected_format, texture_format)
}

pub fn load_image_data(raw_bytes: &[u8], expected_format: Option<ImageFormat>, texture_format: TextureFormat) -> Result<TextureData> {
	use ColorType::*;
	let image = if let Some(expected_format) = expected_format {
		image::load_from_memory_with_format(raw_bytes, expected_format)
	} else {
		image::load_from_memory(raw_bytes)
	}.context("Failed to load image from bytes")?;

	let color = image.color();
	let dimensions = image.dimensions();

	let output_color = match color {
		Bgr8 => Bgra8,
		Rgb8 => Rgba8,
		Rgb16 => Rgba16,
		_ => color
	};

	// Do this check before doing any 3-channel -> 4-channel conversions
	if !color_type_matches_format(output_color, texture_format) {
		bail!("Invalid texture format {:?} for color {:?}", texture_format, color);
	}

	let color_bytes = {
		// Convert 3-channel images to 4-channel
		match color {
			Bgr8 => { image.into_bgra8().into_raw() },
			Rgb8 => { image.into_rgba8().into_raw() },
			// TODO find a safe way to cast Vec<u16> to Vec<u8> directly and avoid an extra Vec?
			Rgb16 => { bytemuck::pod_collect_to_vec(image.into_rgba16().into_raw().as_ref()) },
			_ => { image.into_bytes() }
		}
	};

	Ok(TextureData {
		bytes: color_bytes,
		dimensions: Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: 1
		},
		format: texture_format,
	})
}

pub fn create_texture(device: &Device, queue: &Queue, data: TextureData, usage: TextureUsage, label: Option<&str>) -> Texture {
	let texture = device.create_texture(&TextureDescriptor {
		label,

		size: data.dimensions,
		dimension: TextureDimension::D2,
		format: data.format,

		mip_level_count: 1,
		sample_count: 1,
		usage
	});
	queue.write_texture(
		ImageCopyTexture {
			texture: &texture,
			mip_level: 0,
			origin: Origin3d::ZERO,
		},
		data.bytes.as_ref(),
		ImageDataLayout {
			offset: 0,
			bytes_per_row: NonZeroU32::new(data.dimensions.width * data.format.describe().block_size as u32),
			rows_per_image: NonZeroU32::new(data.dimensions.height)
		},
		data.dimensions
	);
	texture
}

// TODO remove usage param?
pub fn load_spritesheet_from_file<P: Clone + Debug + AsRef<Path>> (
		texture_path: P, metadata_path: P, expected_format: Option<ImageFormat>, texture_format: TextureFormat,
		device: &Device, queue: &Queue, usage: TextureUsage)
			-> Result<(Spritesheet, HashMap<String, usize>)> {
	let metadata_string = read_to_string(metadata_path.clone())?;
	let metadata = ron::from_str::<HashMap<String, SpriteMetadata>>(&metadata_string)?;
	let texture_data = load_image_from_file(texture_path, expected_format, texture_format)?;
	// TODO only use label in debug builds?
	let texture = create_texture(device, queue, texture_data, usage, Some(&*format!("spritesheet {:?}", metadata_path)));
	Ok(Spritesheet::create(texture, None, metadata))
}
