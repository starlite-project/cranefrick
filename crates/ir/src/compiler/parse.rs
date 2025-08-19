use std::{
	io,
	path::{Path, PathBuf},
};

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
	input::{Stream, ValueInput},
	prelude::*,
};
use tracing::{info, trace};

use crate::BrainIr;

#[derive(Debug, Clone)]
pub struct AstParser<'a> {
	file_data: String,
	file_path: &'a Path,
}

impl<'a> AstParser<'a> {
	#[must_use]
	pub const fn new(file_data: String, path: &'a Path) -> Self {
		Self {
			file_data,
			file_path: path,
		}
	}

	pub fn parse(self) -> io::Result<Vec<BrainIr>> {
		info!("got input of {} chars", self.file_data.len());

		let source = Stream::from_iter(
			self.file_data
				.chars()
				.filter(|c| matches!(c, '[' | ']' | '>' | '<' | '+' | '-' | ',' | '.')),
		);

		match parser().parse(source).into_result() {
			Ok(e) => Ok(e),
			Err(errs) => {
				for err in errs {
					Report::build(ReportKind::Error, ((), err.span().into_range()))
						.with_config(
							ariadne::Config::new().with_index_type(ariadne::IndexType::Byte),
						)
						.with_message(err.to_string())
						.with_label(
							Label::new(((), err.span().into_range()))
								.with_message(err.reason().to_string())
								.with_color(Color::Red),
						)
						.finish()
						.eprint(Source::from(&self.file_data))?;
				}
				Ok(Vec::new())
			}
		}
	}
}

fn parser<'src, I>() -> impl Parser<'src, I, Vec<BrainIr>, extra::Err<Rich<'src, char>>>
where
	I: ValueInput<'src, Token = char, Span = SimpleSpan>,
{
	trace!("creating parser-combinator");

	recursive(|bf| {
		choice((
			just('<').to(BrainIr::move_pointer(-1)),
			// just('>').to(BrainIr::move_pointer(1)),
			// just('+').to(BrainIr::change_cell(1)),
			// just('-').to(BrainIr::change_cell(-1)),
			// just(',').to(BrainIr::input_cell()),
			// just('.').to(BrainIr::output_current_cell()),
			// just("[-]").to(BrainIr::clear_cell()),
		))
		.or(bf
			.delimited_by(just('['), just(']'))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
