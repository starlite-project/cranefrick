use std::fmt::{Display, Formatter, Result as FmtResult};

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
	input::{Stream, ValueInput},
	prelude::*,
};
use logos::Logos;

const SRC: &str = r"
    (-
        (* (+ 4 7.3) 7)
        (/ 5 3)
    )
";

fn main() {
	let token_iter = Token::lexer(SRC)
		.spanned()
		.filter_map(|(tok, span)| match tok {
			Ok(tok) => Some((tok, span.into())),
			Err(..) => None,
		});

	let token_stream =
		Stream::from_iter(token_iter).map((0..SRC.len()).into(), |(t, s): (_, _)| (t, s));

	match parser().parse(token_stream).into_result() {
		Ok(sexpr) => match sexpr.eval() {
			Ok(out) => println!("Result = {out}"),
			Err(err) => println!("Runtime error: {err}"),
		},
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
					.eprint(Source::from(SRC))
					.unwrap();
			}
		}
	}
}

#[derive(Clone, PartialEq, Logos)]
#[logos(skip r"[ \t\f\n]+")]
enum Token<'a> {
	#[regex(r"[+-]?([0-9]*[.])?[0-9]+")]
	Float(&'a str),
	#[token("+")]
	Add,
	#[token("-")]
	Sub,
	#[token("*")]
	Mul,
	#[token("/")]
	Div,
	#[token("(")]
	LParen,
	#[token(")")]
	RParen,
}

impl Display for Token<'_> {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Float(s) => write!(f, "{s}"),
			Self::Add => write!(f, "+"),
			Self::Sub => write!(f, "-"),
			Self::Mul => write!(f, "*"),
			Self::Div => write!(f, "/"),
			Self::LParen => write!(f, "("),
			Self::RParen => write!(f, ")"),
		}
	}
}

#[derive(Debug)]
enum SExpr {
	Float(f64),
	Add,
	Sub,
	Mul,
	Div,
	List(Vec<Self>),
}

impl SExpr {
	fn eval(&self) -> Result<f64, &'static str> {
		match self {
			Self::Float(x) => Ok(*x),
			Self::Add | Self::Sub | Self::Mul | Self::Div => Err("Cannot evaluate operator"),
			Self::List(list) => match &list[..] {
				[Self::Add, tail @ ..] => tail.iter().map(Self::eval).sum(),
				[Self::Mul, tail @ ..] => tail.iter().map(Self::eval).product(),
				[Self::Sub, init, tail @ ..] => Ok(init.eval()?
					- tail
						.iter()
						.map(Self::eval)
						.sum::<Result<f64, &'static str>>()?),
				[Self::Div, init, tail @ ..] => {
					Ok(init.eval()? / tail.iter().map(Self::eval).product::<Result<f64, _>>()?)
				}
				_ => Err("Cannot evaluate list"),
			},
		}
	}
}

fn parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, SExpr, extra::Err<Rich<'tokens, Token<'src>>>>
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = SimpleSpan>,
{
	recursive(|sexpr| {
		let atom = select! {
			Token::Float(x) => SExpr::Float(x.parse().unwrap()),
			Token::Add => SExpr::Add,
			Token::Sub => SExpr::Sub,
			Token::Mul => SExpr::Mul,
			Token::Div => SExpr::Div,
		};

		let list = sexpr
			.repeated()
			.collect()
			.map(SExpr::List)
			.delimited_by(just(Token::LParen), just(Token::RParen));

		atom.or(list)
	})
}
