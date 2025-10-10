use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct Args {
	pub file_path: PathBuf,
	#[arg(short, long)]
	pub output_path: PathBuf,
	#[arg(short, long)]
	pub passes_path: Option<PathBuf>,
}

#[allow(unreachable_patterns)]
impl Args {
	pub fn file_path(&self) -> &Path {
		&self.file_path
	}

	pub fn output_path(&self) -> &Path {
		&self.output_path
	}

	pub fn passes_path(&self) -> Option<&Path> {
		self.passes_path.as_deref()
	}
}
