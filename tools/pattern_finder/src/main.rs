use std::{fs, path::PathBuf};

use clap::Parser;
use color_eyre::Result;
use walkdir::WalkDir;

fn main() -> Result<()> {
	color_eyre::install()?;

	let args = match Args::try_parse() {
		Ok(a) => a,
		Err(e) => {
			eprintln!("{e}");
			return Ok(());
		}
	};

	let input = args
		.input
		.chars()
		.filter(|c| is_brainfuck(*c))
		.collect::<String>();

	let files = WalkDir::new(&args.folder_path)
		.into_iter()
		.filter_map(Result::ok)
		.collect::<Vec<_>>();

	println!("searching for {input} in {} files", files.len());

	let mut count = 0;

	for dir_entry in files {
		let Ok(file_contents) = fs::read_to_string(dir_entry.path()) else {
			continue;
		};

		let file_contents = file_contents
			.chars()
			.filter(|c| is_brainfuck(*c))
			.collect::<String>();

		let indices = file_contents.match_indices(&input).collect::<Vec<_>>();

		if !indices.is_empty() {
			println!(
				"found pattern {input} in file {} {} times",
				dir_entry.path().display(),
				indices.len()
			);

			count += indices.len();
		}
	}

	println!("found {input} a total of {count} times");

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	folder_path: PathBuf,
	input: String,
}

const fn is_brainfuck(c: char) -> bool {
	matches!(c, '[' | ']' | '+' | '-' | '.' | ',' | '>' | '<')
}
