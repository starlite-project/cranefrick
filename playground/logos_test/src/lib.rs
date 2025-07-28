#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use logos::Logos;

#[derive(Debug, Logos)]
#[logos(skip r"[ \t\n]+")]
pub enum Token {
	#[token("+")]
	Plus,
	#[token("-")]
	Minus,
	#[token("*")]
	Multiply,
	#[token("/")]
	Divide,
	#[token("(")]
	LParen,
	#[token(")")]
	RParent,
	#[token("[0-9]+", |lex| lex.slice().parse::<isize>().unwrap())]
	Integer(isize),
}
