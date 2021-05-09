// The contents of this file are based on code from Setadokalo's vulkan engine
// https://gitlab.com/Setadokalo/vulkan-engine/-/blob/master/src/asset_loading/texture.rs
// and https://gitlab.com/Setadokalo/vulkan-engine/-/blob/master/src/asset_loading/offloaded_texture.rs
// licensed under the MIT license:
// MIT License
//
// Copyright (c) 2020 Greg Stancil
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use vulkano::device::Queue;
use vulkano::format::Format;

use image::{ColorType, ImageDecoder};
use regex::Regex;

#[derive(Debug)]
pub struct RawTextureData {
	pub bytes: Vec<u8>,
	pub size: (u32, u32),
	pub format: Format,
	queue: Option<Arc<Queue>>,
}

impl RawTextureData {
	pub fn set_queue(&mut self, queue: Arc<Queue>) {
		self.queue = Some(queue);
	}
	pub fn new(bytes: Vec<u8>, size: (u32, u32), format: Format, queue: Option<Arc<Queue>>) -> Self {
		Self {
			bytes,
			size,
			format,
			queue,
		}
	}
}

#[derive(Debug)]
pub enum LoadError {
	FileLoadError(std::io::Error),
	MalformedFile,
}
// impl From<ObjError> for LoadError {
// 	fn from(obj: ObjError) -> Self {
// 		match obj {
// 			ObjError::Io(err) => LoadError::FileLoadError(err),
// 			_ => LoadError::MalformedFile,
// 		}
// 	}
// }

impl From<std::io::Error> for LoadError {
	fn from(err: std::io::Error) -> Self {
		LoadError::FileLoadError(err)
	}
}


/// Converts `ColorType`s to their appropriate matching `Format`.
///
/// Note: the returned `Format` is the closest matching universally-accepted format, not necessarily the exact same bits-per-pixel.
/// Namely, any RGB/BGR format without an alpha channel will need to have one zippered on.
/// # Panics
/// This method panics if it doesn't recognize the ColorType.
fn convert_ctype_to_format(ctype: ColorType, prefer_srgb: bool) -> Format {
	match ctype {
		ColorType::L8 => Format::R8Unorm,
		ColorType::La8 => Format::R8G8Unorm,
		ColorType::Rgb8 => {
			if prefer_srgb {
				Format::R8G8B8A8Srgb
			} else {
				Format::R8G8B8A8Unorm
			}
		}
		ColorType::Rgba8 => {
			if prefer_srgb {
				Format::R8G8B8A8Srgb
			} else {
				Format::R8G8B8A8Unorm
			}
		}
		ColorType::L16 => Format::R16Sfloat,
		ColorType::La16 => Format::R16G16Sfloat,
		ColorType::Rgb16 => Format::R16G16B16A16Sfloat,
		ColorType::Rgba16 => Format::R16G16B16A16Sfloat,
		ColorType::Bgr8 => {
			if prefer_srgb {
				Format::B8G8R8A8Srgb
			} else {
				Format::B8G8R8A8Unorm
			}
		}
		ColorType::Bgra8 => {
			if prefer_srgb {
				Format::B8G8R8A8Srgb
			} else {
				Format::B8G8R8A8Unorm
			}
		}
		ColorType::__NonExhaustive(_) => panic!("Unrecognized color type!"),
	}
}



pub fn load_texture(path: String, prefer_srgb: bool) -> RawTextureData {
	let start_time = std::time::Instant::now();
	let mut dfile = File::open(path.clone()).expect("Failed to open file");
	// glium::Texture2d::new(facade, data).expect("Failed to generate texture")
	let (mut data, color_type, dimensions) = if path.ends_with(".png") {
		load_texture_png(&mut dfile)
	} else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
		load_texture_jpg(&mut dfile)
	} else if path.ends_with(".ff") || path.ends_with(".farbfeld") {
		//.farbfeld is technically invalid but I bet some people use it
		load_texture_ff(&mut dfile)
	} else {
		panic!("Invalid texture type!");
	}
		.expect("File load error");
	// It doesn't matter if it's BGR or RGB, if the bpp is 24 we need to add an alpha channel
	if color_type.bits_per_pixel() == 24 {
		data = add_alpha_channel(data);
	} else if color_type.bits_per_pixel() == 48 {
		data = add_alpha_channel_16(data);
	}
	// TODO probably shouldn't be recreating this each time - Seta caches it, I think?
	let r = Regex::new(r"(.*/)*(.*?\..*?$)").expect("Failed to build regex");
	let filename_opt = r.captures(&*path);
	if let Some(filename_cap) = filename_opt {
		if let Some(filename) = filename_cap.get(2) {
			println!(
				"[Image Decoder] Returning image \"{}\"; took {:.4} seconds",
				filename.as_str(),
				start_time.elapsed().as_secs_f64()
			);
		} else {
			println!(
				"[Image Decoder] Returning constructed image; took {:.4} seconds",
				start_time.elapsed().as_secs_f64()
			);
		}
	} else {
		println!(
			"[Image Decoder] Returning constructed image; took {:.4} seconds",
			start_time.elapsed().as_secs_f64()
		);
	}

	RawTextureData::new(
		data,
		dimensions,
		convert_ctype_to_format(color_type, prefer_srgb),
		None,
	)
}

pub fn load_texture_png(dfile: &mut File) -> Result<(Vec<u8>, ColorType, (u32, u32)), LoadError> {
	let mut raw_data_buf = Vec::new();
	dfile.read_to_end(&mut raw_data_buf)?;
	let img = image::png::PngDecoder::new(&raw_data_buf[..]).expect("Failed to load image file");
	let color_type = img.color_type();
	let total_bytes = img.total_bytes() as usize;
	let mut data: Vec<u8> = vec![0u8; total_bytes];
	let dimensions = img.dimensions();
	img.read_image(&mut data[..])
		.expect("Failed to convert image to raw data");
	Ok((data, color_type, dimensions))
}

// This function zippers an alpha channel onto 8-bpc textures, since GPUs almost never support 24-bit color.
fn add_alpha_channel(opaque_data: Vec<u8>) -> Vec<u8> {
	// this is ugly and probably very bad code, according to Seta
	let total_bytes = opaque_data.len();
	let mut data: Vec<u8> = vec![255u8; total_bytes / 3 * 4];
	let mut bytes_offset = 0_usize;
	for i in 0..total_bytes {
		data[i + bytes_offset] = opaque_data[i];
		if i % 3 == 2 {
			bytes_offset = bytes_offset + 1;
		}
	}
	data
}

// WARNING: THIS IS COMPLETELY UNTESTED. Thanks Seta very cool.
// Use at your own risk.
// This function zippers an alpha channel onto 16-bpc textures, since GPUs almost never support 48-bit color.
fn add_alpha_channel_16(opaque_data: Vec<u8>) -> Vec<u8> {
	// this is ugly and probably very bad code, again
	let total_bytes = opaque_data.len();
	let mut data: Vec<u8> = vec![255u8; total_bytes / 3 * 4];
	let mut bytes_offset = 0_usize;
	for i in 0..total_bytes {
		data[i + bytes_offset] = opaque_data[i];
		if i % 6 == 5 {
			bytes_offset = bytes_offset + 2;
		}
	}
	data
}

pub fn load_texture_jpg(dfile: &mut File) -> Result<(Vec<u8>, ColorType, (u32, u32)), LoadError> {
	let mut raw_data_buf = Vec::new();
	dfile.read_to_end(&mut raw_data_buf)?;
	let img = image::jpeg::JpegDecoder::new(&raw_data_buf[..]).expect("Failed to load image file");
	let color_type = img.color_type();
	let total_bytes = img.total_bytes() as usize;
	// since this is a jpg, the data loaded in will always be RGB; the GPU will only accept RGBA so we need to zipper in an alpha channel
	let mut data: Vec<u8> = vec![0u8; total_bytes];
	let dimensions = img.dimensions();
	img.read_image(&mut data[..])
		.expect("Failed to convert image to raw data");
	Ok((data, color_type, dimensions))
}

pub fn load_texture_ff(dfile: &mut File) -> Result<(Vec<u8>, ColorType, (u32, u32)), LoadError> {
	let mut raw_data_buf = Vec::new();
	dfile.read_to_end(&mut raw_data_buf)?;
	let img =
		image::farbfeld::FarbfeldDecoder::new(&raw_data_buf[..]).expect("Failed to load image file");
	let color_type = img.color_type();
	let mut data: Vec<u8> = vec![0u8; img.total_bytes() as usize];
	let dimensions = img.dimensions();
	img.read_image(&mut data[..])
		.expect("Failed to convert image to raw data");
	Ok((data, color_type, dimensions))
}
