use alloc::{string::ToString as _, vec::Vec};
use std::{io, path::Path};

use ariadne::{Color, IndexType, Label, Report, ReportKind, Source};
use chumsky::prelude::*;

use crate::{BrainOperation, BrainOperationType};

pub fn parse(file_path: impl AsRef<Path>) -> io::Result<Vec<BrainOperation>> {
	let file_path = file_path.as_ref();

	let file_name = file_path.file_name().and_then(|s| s.to_str()).unwrap();

	let file_data = std::fs::read_to_string(file_path)?;

	match parser().parse(file_data.as_str()).into_result() {
		Ok(e) => Ok(e),
		Err(errs) => {
			for err in errs {
				Report::build(ReportKind::Error, (file_name, err.span().into_range()))
					.with_config(ariadne::Config::new().with_index_type(IndexType::Byte))
					.with_message(err.to_string())
					.with_label(
						Label::new((file_name, err.span().into_range()))
							.with_message(err.reason().to_string())
							.with_color(Color::Red),
					)
					.finish()
					.eprint((file_name, Source::from(&file_data)))?;
			}

			Ok(Vec::new())
		}
	}
}

fn parser<'src>() -> impl Parser<'src, &'src str, Vec<BrainOperation>, extra::Err<Rich<'src, char>>>
{
	recursive(|expr| {
		choice((
			just('+')
				.to(BrainOperationType::ChangeCell(1))
				.labelled("increment"),
			just('-')
				.to(BrainOperationType::ChangeCell(-1))
				.labelled("decrement"),
			just('<')
				.to(BrainOperationType::MovePointer(-1))
				.labelled("move left"),
			just('>')
				.to(BrainOperationType::MovePointer(1))
				.labelled("move right"),
			just('.')
				.to(BrainOperationType::OutputCurrentCell)
				.labelled("output"),
			just(',')
				.to(BrainOperationType::InputIntoCell)
				.labelled("input"),
			none_of("+-<>.,[]").map(|x: char| BrainOperationType::Comment(x.to_string())),
		))
		.or(expr
			.delimited_by(
				just('[').labelled("start loop"),
				just(']').labelled("end loop"),
			)
			.map(BrainOperationType::DynamicLoop))
		.map_with(|e, t| {
			BrainOperation::new(e, {
				let span: SimpleSpan = t.span();

				span.into_range()
			})
		})
		.repeated()
		.collect()
	})
}
