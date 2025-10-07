use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
	pub folder_path: PathBuf,
	pub input: String,
}
