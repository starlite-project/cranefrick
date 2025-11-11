use alloc::{
	string::{String, ToString as _},
	vec::Vec,
};
use std::io;

use ariadne::{Color, IndexType, Label, Report, ReportKind, Source};
use chumsky::prelude::*;

use crate::{BrainOperation, BrainOperationType};

pub fn parse(file_data: String) -> io::Result<Vec<BrainOperation>> {
	match parser().parse(file_data.as_str()).into_result() {
		Ok(e) => Ok(e),
		Err(errs) => {
			for err in errs {
				Report::build(ReportKind::Error, ((), err.span().into_range()))
					.with_config(ariadne::Config::new().with_index_type(IndexType::Byte))
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

fn parser<'src>() -> impl Parser<'src, &'src str, Vec<BrainOperation>, extra::Err<Rich<'src, char>>>
{
	recursive(|expr| {
		choice((
			just('+').to(BrainOperationType::ChangeCell(1)),
			just('-').to(BrainOperationType::ChangeCell(-1)),
			just('<').to(BrainOperationType::MovePointer(-1)),
			just('>').to(BrainOperationType::MovePointer(1)),
			just('.').to(BrainOperationType::OutputCurrentCell),
			just(',').to(BrainOperationType::InputIntoCell),
		))
		.or(expr
			.delimited_by(just('['), just(']'))
			.map(BrainOperationType::DynamicLoop))
		.map_with(|e, t| {
			BrainOperation::new(e, {
				let span: SimpleSpan = t.span();

				span.into_range()
			})
		})
		.padded()
		.repeated()
		.collect()
	})
}
