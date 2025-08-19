#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

use core::fmt::{Display, Formatter, Result as FmtResult};

pub use logos::Logos;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Logos)]
pub enum BrainAst {
	#[token("<")]
	MovePtrLeft,
	#[token(">")]
	MovePtrRight,
	#[token("+")]
	IncrementCell,
	#[token("-")]
	DecrementCell,
	#[token(",")]
	GetInput,
	#[token(".")]
	PutOutput,
	#[token("[")]
	StartLoop,
	#[token("]")]
	EndLoop,
	#[token("[-]")]
	ClearCell,
}

impl Display for BrainAst {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match *self {
			Self::MovePtrLeft => "<",
			Self::MovePtrRight => ">",
			Self::IncrementCell => "+",
			Self::DecrementCell => "-",
			Self::GetInput => ",",
			Self::PutOutput => ".",
			Self::StartLoop => "[",
			Self::EndLoop => "]",
			Self::ClearCell => "[-]",
		})
	}
}
