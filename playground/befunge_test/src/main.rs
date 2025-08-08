use std::{
	fs,
	io::{BufRead as _, BufReader},
	path::PathBuf,
};

use befunge_test::{CellInt, Direction, create_cfg, execute};
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

	let mut code: [CellInt; 2560] = [0; 2560];
	let stack: [CellInt; 4096] = [0; 4096];
	let stack_idx = -1isize;

	{
		let fin = fs::File::open(&args.file_path)?;
		let reader = BufReader::new(fin);
		for x in 0..80usize {
			for y in 0..25usize {
				code[x << 5 | y] = 32;
			}
		}

		for (y, line) in reader.lines().take(25).enumerate() {
			if let Ok(line) = line {
				for (x, ch) in line
					.chars()
					.take(80)
					.take_while(|&c| !matches!(c, '\n'))
					.enumerate()
				{
					code[x << 5 | y] = CellInt::from(ch as u32);
				}
			}
		}
	}

	let mut iteration = 0;
	let mut xy = 0;
	let mut dir = Direction::Right;

	loop {
		let mut progbits = [0u8; 320];
		let cfg = create_cfg(&code, &mut progbits, xy, dir);

		let newxydir = execute(&cfg, &progbits, &code, &stack, &stack_idx, iteration)?;

		if matches!(newxydir, u32::MAX) {
			break;
		}

		xy = newxydir >> 2;
		dir = newxydir.into();
		iteration += 1;
	}

	Ok(())
}

#[derive(Debug, Parser)]
struct Args {
	file_path: PathBuf,
}
