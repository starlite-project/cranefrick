use std::io;

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
	input::{Stream, ValueInput},
	prelude::*,
};
use cranefrick_ast::BrainAst;

use crate::BrainIr;

#[derive(Debug, Clone)]
pub struct AstParser {
	source: Vec<(BrainAst, SimpleSpan)>,
	file_data: String,
}

impl AstParser {
	pub fn new(
		source: impl IntoIterator<Item = (BrainAst, SimpleSpan)>,
		file_data: String,
	) -> Self {
		Self {
			source: source.into_iter().collect(),
			file_data,
		}
	}

	pub fn parse(self) -> io::Result<Vec<BrainIr>> {
		let token_stream = Stream::from_iter(self.source)
			.map((0..self.file_data.len()).into(), |(t, s): (_, _)| (t, s));

		match parser().parse(token_stream).into_result() {
			Ok(e) => Ok(e),
			Err(errs) => {
				for err in errs {
					Report::build(ReportKind::Error, ((), err.span().into_range()))
						.with_config(
							ariadne::Config::new().with_index_type(ariadne::IndexType::Byte),
						)
						.with_code(3)
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

fn parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<BrainIr>, extra::Err<Rich<'tokens, BrainAst>>>
where
	I: ValueInput<'tokens, Token = BrainAst, Span = SimpleSpan>,
{
	recursive(|bf| {
		choice((
			just(BrainAst::MovePtrLeft).to(BrainIr::move_pointer(-1)),
			just(BrainAst::MovePtrRight).to(BrainIr::move_pointer(1)),
			just(BrainAst::IncrementCell).to(BrainIr::change_cell(1)),
			just(BrainAst::DecrementCell).to(BrainIr::change_cell(-1)),
			just(BrainAst::GetInput).to(BrainIr::input_cell()),
			just(BrainAst::PutOutput).to(BrainIr::output_current_cell()),
		))
		.or(bf
			.delimited_by(just(BrainAst::StartLoop), just(BrainAst::EndLoop))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
