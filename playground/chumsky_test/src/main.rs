use std::fmt::{Display, Formatter, Result as FmtResult};

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
	input::{Stream, ValueInput},
	prelude::*,
};
use logos::Logos;

fn main() {}

enum BrainMlir {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
enum BrainHlir {
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
