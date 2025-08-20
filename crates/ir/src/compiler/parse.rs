use std::io;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{input::ValueInput, prelude::*};
use tracing::{info, trace};

use crate::BrainIr;

#[derive(Debug, Clone)]
pub struct AstParser {
	file_data: String,
}

impl AstParser {
	#[must_use]
	pub const fn new(file_data: String) -> Self {
		Self { file_data }
	}

	pub fn parse(self) -> io::Result<Vec<BrainIr>> {
		info!("got input of {} chars", self.file_data.len());

		match parser().parse(self.file_data.as_str()).into_result() {
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
			// noop patterns
			just("><").or(just("<>")).to(BrainIr::move_pointer(0)),
			just("+-").to(BrainIr::change_cell(0)),
			// common patterns
			just("[-]").to(BrainIr::clear_cell()),
			// basics
			just('+').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::change_cell(span.into_iter().len() as i8)
			}),
			just('-').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::change_cell(-(span.into_iter().len() as i8))
			}),
			just('>').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::move_pointer(span.into_iter().len() as i32)
			}),
			just('<').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::move_pointer(-(span.into_iter().len() as i32))
			}),
			just(',').repeated().at_least(1).to(BrainIr::input_cell()),
			just('.').to(BrainIr::output_current_cell()),
		))
		.or(bf
			.delimited_by(just('['), just(']'))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
