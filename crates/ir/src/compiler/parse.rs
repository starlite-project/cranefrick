use std::io;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use tracing::trace;

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
		match parser().parse(&self.file_data).into_result() {
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
				// for err in errs {
				// 	Report::build(ReportKind::Error, ((), err.span().into_range()))
				// 		.with_config(
				// 			ariadne::Config::new().with_index_type(ariadne::IndexType::Byte),
				// 		)
				// 		.with_code(3)
				// 		.with_message(err.to_string())
				// 		.with_label(
				// 			Label::new(((), err.span().into_range()))
				// 				.with_message(err.reason().to_string())
				// 				.with_color(Color::Red),
				// 		)
				// 		.finish()
				// 		.eprint(Source::from(&self.file_data))?;
				// }
				Ok(Vec::new())
			}
		}
	}
}

fn parser<'src>() -> impl Parser<'src, &'src str, Vec<BrainIr>, extra::Err<Rich<'src, char>>> {
	trace!("creating parser-combinator");

	recursive(|bf| {
		choice((
			just('<').to(BrainIr::move_pointer(-1)),
			just('>').to(BrainIr::move_pointer(1)),
			just('+').to(BrainIr::change_cell(1)),
			just('-').to(BrainIr::change_cell(-1)),
			just(',').to(BrainIr::input_cell()),
			just('.').to(BrainIr::output_current_cell()),
		))
		.or(bf
			.delimited_by(just('['), just(']'))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
