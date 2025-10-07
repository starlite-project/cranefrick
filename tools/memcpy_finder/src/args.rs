use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
	pub file_path: PathBuf,
    #[arg(long)]
	pub summary: bool,
}
