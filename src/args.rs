use std::path::PathBuf;

use clap::{Parser, ValueEnum, builder::PossibleValue};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
	pub file_path: PathBuf,
	#[arg(short, long)]
	pub output_path: PathBuf,
	#[arg(short, long)]
	pub flags_path: Option<PathBuf>,
	#[arg(short, long, default_value = "cranelift")]
	pub assembler: Assembler,
}

#[derive(Debug, Clone, Copy)]
pub enum Assembler {
	Cranelift,
	#[cfg(feature = "llvm")]
	Llvm,
}

impl ValueEnum for Assembler {
	fn value_variants<'a>() -> &'a [Self] {
		&[
			Self::Cranelift,
			#[cfg(feature = "llvm")]
			Self::Llvm,
		]
	}

	fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
		Some(PossibleValue::new(match *self {
			Self::Cranelift => "cranelift",
			#[cfg(feature = "llvm")]
			Self::Llvm => "llvm",
		}))
	}
}
