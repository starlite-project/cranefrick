use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub enum Args {
	#[cfg(feature = "cranelift")]
	Cranelift {
		file_path: PathBuf,
		#[arg(short, long)]
		output_path: PathBuf,
		#[arg(short, long)]
		flags_path: Option<PathBuf>,
	},
	#[cfg(feature = "llvm")]
	Llvm {
		file_path: PathBuf,
		#[arg(short, long)]
		output_path: PathBuf,
		#[arg(short, long)]
		passes_path: Option<PathBuf>,
	},
}

#[allow(unreachable_patterns)]
impl Args {
	pub fn file_path(&self) -> &Path {
		match self {
			#[cfg(feature = "cranelift")]
			Self::Cranelift { file_path, .. } => file_path,
			#[cfg(feature = "llvm")]
			Self::Llvm { file_path, .. } => file_path,
			_ => unreachable!(),
		}
	}

	pub fn output_path(&self) -> &Path {
		match self {
			#[cfg(feature = "cranelift")]
			Self::Cranelift { output_path, .. } => output_path,
			#[cfg(feature = "llvm")]
			Self::Llvm { output_path, .. } => output_path,
			_ => unreachable!(),
		}
	}
}
