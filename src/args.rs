use std::path::{Path, PathBuf};

use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub enum Args {
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
		passes: Option<String>,
	},
}

impl Args {
	pub fn file_path(&self) -> &Path {
		match self {
			Self::Cranelift { file_path, .. } | Self::Llvm { file_path, .. } => file_path,
		}
	}

	pub fn output_path(&self) -> &Path {
		match self {
			Self::Cranelift { output_path, .. } | Self::Llvm { output_path, .. } => output_path,
		}
	}
}

// #[derive(Debug, Parser)]
// #[command(version, about, long_about = None)]
// pub struct Args {
// 	pub file_path: PathBuf,
// 	#[arg(short, long)]
// 	pub output_path: PathBuf,
// 	#[arg(short, long)]
// 	pub flags_path: Option<PathBuf>,
// 	#[arg(short, long, default_value = "cranelift")]
// 	pub assembler: AssemblerType,
// }

// #[derive(Debug, Clone, ValueEnum
// )]
// pub enum AssemblerType {
// 	Cranelift {
// 		flags_path: Option<PathBuf>
// 	},
// 	Llvm {
// 		passes: Option<String>
// 	}
// }
