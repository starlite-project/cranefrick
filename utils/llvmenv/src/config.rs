use std::{fs, io::Write, path::PathBuf};

use tracing::info;

use super::{Error, FileIoConvert as _, Result};

pub const APP_NAME: &str = "llvmenv";
pub const ENTRY_TOML: &str = "entry.toml";

const LLVM_MIRROR: &str = include_str!("llvm-mirror.toml");

pub fn config_dir() -> Result<PathBuf> {
	ensure_dir_exists(dirs::config_dir())
}

pub fn cache_dir() -> Result<PathBuf> {
	ensure_dir_exists(dirs::cache_dir())
}

pub fn data_dir() -> Result<PathBuf> {
	ensure_dir_exists(dirs::data_dir())
}

pub fn init_config() -> Result<()> {
	let dir = config_dir()?;
	let entry = dir.join(ENTRY_TOML);

	if entry.exists() {
		Err(Error::ConfigureAlreadyExists { path: entry })
	} else {
		info!("create default entry setting: {}", entry.display());
		let mut f = fs::File::create(&entry).with(&entry)?;
		f.write(LLVM_MIRROR.as_bytes()).with(&entry)?;
		Ok(())
	}
}

fn ensure_dir_exists(path: Option<PathBuf>) -> Result<PathBuf> {
	let path = path.ok_or(Error::UnsupportedOs)?.join(APP_NAME);

	if !path.exists() {
		fs::create_dir_all(&path).with(&path)?;
	}

	Ok(path)
}
