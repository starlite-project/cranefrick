use std::io;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{input::ValueInput, prelude::*};
use tracing::{info, trace};

use super::BrainIr;

#[tracing::instrument(skip_all)]
pub fn parse(file_data: String) -> io::Result<Vec<BrainIr>> {
	info!("got input of {} chars", file_data.len());

	match parser().parse(file_data.as_str()).into_result() {
		Ok(e) => Ok(e),
		Err(errs) => {
			for err in errs {
				Report::build(ReportKind::Error, ((), err.span().into_range()))
					.with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
					.with_message(err.to_string())
					.with_label(
						Label::new(((), err.span().into_range()))
							.with_message(err.reason().to_string())
							.with_color(Color::Red),
					)
					.finish()
					.eprint(Source::from(&file_data))?;
			}
			Ok(Vec::new())
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
			just("+-").or(just("-+")).to(BrainIr::change_cell(0)),
			// common patterns
			just("[-]").to(BrainIr::clear_cell()),
			// basics
			just('+').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::change_cell(span.into_iter().len() as i8)
			}),
			just('-').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::change_cell((span.into_iter().len() as i8).wrapping_neg())
			}),
			just('>').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::move_pointer(span.into_iter().len() as i32)
			}),
			just('<').repeated().at_least(1).map_with(|(), e| {
				let span: SimpleSpan = e.span();

				BrainIr::move_pointer((span.into_iter().len() as i32).wrapping_neg())
			}),
			just(',').repeated().at_least(1).to(BrainIr::input_cell()),
			just('.').to(BrainIr::output_cell()),
		))
		.or(bf
			.delimited_by(just('['), just(']'))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
