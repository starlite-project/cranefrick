use std::{
	fmt::{Display, Formatter, Result as FmtResult},
	fs,
	path::PathBuf,
};

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
	input::{Stream, ValueInput},
	prelude::*,
};
use clap::Parser as ClapParser;
use cranefrick_ir::BrainIr;
use logos::Logos;

fn main() {
	let args = Args::parse();

	let file_data = fs::read_to_string(&args.file_path).expect("failed to read file");

	let token_iter = BrainAst::lexer(&file_data)
		.spanned()
		.filter_map(|(tok, span)| Some((tok.ok()?, span.into())));

	let token_stream =
		Stream::from_iter(token_iter).map((0..file_data.len()).into(), |(t, s): (_, _)| (t, s));

	match parser().parse(token_stream).into_result() {
		Ok(e) => println!("{e:?}"),
		Err(errs) => {
			for err in errs {
				Report::build(ReportKind::Error, ((), err.span().into_range()))
					.with_config(ariadne::Config::new().with_index_type(ariadne::IndexType::Byte))
					.with_code(3)
					.with_message(err.to_string())
					.with_label(
						Label::new(((), err.span().into_range()))
							.with_message(err.reason().to_string())
							.with_color(Color::Red),
					)
					.finish()
					.eprint(Source::from(&file_data))
					.unwrap();
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
enum BrainAst {
	#[token("<")]
	MoveLeft,
	#[token(">")]
	MoveRight,
	#[token("+")]
	Increment,
	#[token("-")]
	Decrement,
	#[token(",")]
	Input,
	#[token(".")]
	Output,
	#[token("[")]
	StartLoop,
	#[token("]")]
	EndLoop,
	#[token("[-]")]
	Clear,
}

impl Display for BrainAst {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match *self {
			Self::Clear => "[-]",
			Self::MoveLeft => "<",
			Self::MoveRight => ">",
			Self::Increment => "+",
			Self::Decrement => "-",
			Self::Input => ",",
			Self::Output => ".",
			Self::StartLoop => "[",
			Self::EndLoop => "]",
		})
	}
}

#[derive(Debug, ClapParser)]
struct Args {
	file_path: PathBuf,
}

fn parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Vec<BrainIr>, extra::Err<Rich<'tokens, BrainAst>>>
where
	I: ValueInput<'tokens, Token = BrainAst, Span = SimpleSpan>,
{
	#[allow(clippy::enum_glob_use)]
	use BrainAst::*;

	recursive(|bf| {
		choice((
			just(MoveLeft).to(BrainIr::change_cell(-1)),
			just(MoveRight).to(BrainIr::change_cell(1)),
			just(Increment).to(BrainIr::change_cell(1)),
			just(Decrement).to(BrainIr::change_cell(-1)),
			just(Input).to(BrainIr::input_cell()),
			just(Output).to(BrainIr::output_current_cell()),
			just(Clear).to(BrainIr::set_cell(0)),
		))
		.or(bf
			.delimited_by(just(StartLoop), just(EndLoop))
			.map(BrainIr::DynamicLoop))
		.repeated()
		.collect()
	})
}
