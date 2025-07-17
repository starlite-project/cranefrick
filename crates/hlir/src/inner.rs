use logos::Logos;

/// This is here so we don't leak the [`Logos`] implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
pub enum InnerOpCode {
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
}
