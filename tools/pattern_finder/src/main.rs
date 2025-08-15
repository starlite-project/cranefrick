use std::{fs, path::PathBuf};

use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let files = fs::read_dir(&args.folder_path)?.into_iter().collect::<Result<Vec<_>, _>>()?;

	println!("searching for {} in {} files", args.input, files.len());

	for dir_entry in files {
		let Ok(file_contents) = fs::read_to_string(dir_entry.path()) else {
			continue;
		};

		let file_contents = file_contents
			.chars()
			.filter(|c| matches!(c, '[' | ']' | '+' | '-' | '.' | ',' | '>' | '<'))
			.collect::<String>();

		if file_contents.match_indices(&args.input).next().is_some() {
			println!(
				"found pattern {} in file {}",
				args.input,
				dir_entry.path().display()
			);
		}
	}

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	folder_path: PathBuf,
	input: String,
}
