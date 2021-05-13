// The contents of this file are adapted from Sotrh's tutorial
// see https://github.com/sotrh/learn-wgpu/blob/e0cd2b0a3992cf5d00e0b54756206e50430032d6/code/beginner/tutorial3-pipeline/build.rs
// and https://github.com/sotrh/learn-wgpu/blob/e0cd2b0a3992cf5d00e0b54756206e50430032d6/code/intermediate/tutorial13-threading/build.rs
// License:
// MIT License
//
// Copyright (c) 2020 Benjamin Hansen
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

use std::env;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;

use anyhow::*;
use glob::glob;
use rayon::prelude::*;

struct ShaderData {
	src: String,
	src_path: PathBuf,
	spv_path: PathBuf,
	kind: shaderc::ShaderKind,
}

impl ShaderData {
	pub fn load(src_path: PathBuf) -> Result<Self> {
		let extension = src_path
			.extension()
			.context("File has no extension")?
			.to_str()
			.context("Extension cannot be converted to &str")?;
		let kind = match extension {
			"vert" => shaderc::ShaderKind::Vertex,
			"frag" => shaderc::ShaderKind::Fragment,
			"comp" => shaderc::ShaderKind::Compute,
			_ => bail!("Unsupported shader: {}", src_path.display()),
		};

		let src = read_to_string(src_path.clone())?;
		let spv_path = src_path.strip_prefix("src/shaders/")?.with_extension(format!("{}.spv", extension));

		Ok(Self {
			src,
			src_path,
			spv_path,
			kind,
		})
	}
}

pub fn compile_shaders() -> Result<()> {
	let out_dir_path: PathBuf = PathBuf::from(env::var("OUT_DIR")?).join("shaders");

	let mut shader_paths = Vec::new();
	shader_paths.extend(glob("./src/shaders/**/*.vert")?);
	shader_paths.extend(glob("./src/shaders/**/*.frag")?);
	shader_paths.extend(glob("./src/shaders/**/*.comp")?);

	let shaders = shader_paths
		.into_par_iter()
		.map(|glob_result| ShaderData::load(glob_result?))
		.collect::<Vec<Result<_>>>()
		.into_iter()
		.collect::<Result<Vec<_>>>()?;

	let mut compiler = shaderc::Compiler::new().context("Unable to create shader compiler")?;

	// TODO optimize this to only compile individual shaders that have changed?
	for shader in shaders {
		println!("cargo:rerun-if-changed={}", shader.src_path.as_os_str().to_str().unwrap());

		let compiled = compiler.compile_into_spirv(
			&shader.src,
			shader.kind,
			&shader.src_path.to_str().unwrap(),
			"main",
			None,
		)?;

		let out_path = out_dir_path.join(shader.spv_path);
		create_dir_all(out_path.parent().unwrap())?;
		write(out_path, compiled.as_binary_u8())?;
	}

	Ok(())
}
