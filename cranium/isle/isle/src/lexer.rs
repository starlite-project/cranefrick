use std::borrow::Cow;

use super::{
	error::{Error, Span},
	files::Files,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
	pub file: usize,
	pub offset: usize,
}

impl Pos {
	#[must_use]
	pub const fn new(file: usize, offset: usize) -> Self {
		Self { file, offset }
	}

	#[must_use]
	pub fn pretty_print_line(self, files: &Files) -> String {
		format!(
			"{} line {}",
			files.file_name(self.file).unwrap(),
			files.file_line_map(self.file).unwrap().line(self.offset)
		)
	}
}

#[derive(Debug, Clone)]
pub struct Lexer<'src> {
	source: &'src str,
	pos: Pos,
	lookahead: Option<(Pos, Token)>,
}

impl<'src> Lexer<'src> {
	pub fn new(file: usize, src: &'src str) -> Result<Self> {
		let mut this = Self {
			source: src,
			pos: Pos::new(file, 0),
			lookahead: None,
		};

		this.reload()?;
		Ok(this)
	}

	#[must_use]
	pub const fn pos(&self) -> Pos {
		self.pos
	}

	const fn advance_by(&mut self, n: usize) {
		self.pos.offset += n;
	}

	const fn advance(&mut self) {
		self.advance_by(1);
	}

	#[allow(clippy::unused_self)]
	fn error(&self, pos: Pos, message: impl Into<String>) -> Error {
		Error::Parse {
			message: message.into(),
			span: Span::from_single(pos),
		}
	}

	fn next_token(&mut self) -> Result<Option<(Pos, Token)>> {
		const fn is_sym_first_char(c: u8) -> bool {
			match c {
				b'-' | b'0'..=b'9' | b'(' | b')' | b';' => false,
				c if c.is_ascii_whitespace() => false,
				_ => true,
			}
		}

		const fn is_sym_other_char(c: u8) -> bool {
			match c {
				b'(' | b')' | b';' | b'@' => false,
				c if c.is_ascii_whitespace() => false,
				_ => true,
			}
		}

		while let Some(c) = self.peek_byte() {
			match c {
				b' ' | b'\t' | b'\n' | b'\r' => self.advance(),
				b';' => {
					while let Some(c) = self.peek_byte() {
						match c {
							b'\n' | b'\r' => break,
							_ => self.advance(),
						}
					}
				}
				b'(' if matches!(self.lookahead_byte(1), Some(b';')) => {
					let pos = self.pos();
					self.advance_by(2);
					let mut depth = 1usize;
					loop {
						match self.peek_byte() {
							None => return Err(self.error(pos, "unterminated block comment")),
							Some(b'(') if matches!(self.lookahead_byte(1), Some(b';')) => {
								self.advance_by(2);
								depth += 1;
							}
							Some(b';') if matches!(self.lookahead_byte(1), Some(b')')) => {
								self.advance_by(2);
								depth -= 1;
								if matches!(depth, 0) {
									break;
								}
							}
							Some(..) => self.advance(),
						}
					}
				}
				_ => break,
			}
		}

		let Some(c) = self.peek_byte() else {
			return Ok(None);
		};

		let char_pos = self.pos();
		match c {
			b'(' => {
				self.advance();
				Ok(Some((char_pos, Token::LParen)))
			}
			b')' => {
				self.advance();
				Ok(Some((char_pos, Token::RParen)))
			}
			b'@' => {
				self.advance();
				Ok(Some((char_pos, Token::At)))
			}
			c if is_sym_first_char(c) => {
				let start = self.pos.offset;
				let start_pos = self.pos();
				while let Some(c) = self.peek_byte() {
					match c {
						c if is_sym_other_char(c) => self.advance(),
						_ => break,
					}
				}
				let end = self.pos.offset;
				let s = &self.source[start..end];
				debug_assert!(!s.is_empty());
				Ok(Some((start_pos, Token::Symbol(s.to_owned()))))
			}
			c @ (b'0'..=b'9' | b'-') => {
				let start_pos = self.pos();
				let neg = if matches!(c, b'-') {
					self.advance();
					true
				} else {
					false
				};

				let mut radix = 10;

				match (
					self.source.as_bytes().get(self.pos.offset),
					self.source.as_bytes().get(self.pos.offset + 1),
				) {
					(Some(b'0'), Some(b'x' | b'X')) => {
						self.advance_by(2);
						radix = 16;
					}
					(Some(b'0'), Some(b'o' | b'O')) => {
						self.advance_by(2);
						radix = 8;
					}
					(Some(b'0'), Some(b'b' | b'B')) => {
						self.advance_by(2);
						radix = 2;
					}
					_ => {}
				}

				let start = self.pos.offset;
				while let Some(c) = self.peek_byte() {
					match c {
						b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'_' => self.advance(),
						_ => break,
					}
				}

				let end = self.pos.offset;
				let s = &self.source[start..end];
				let s = if s.contains('_') {
					Cow::Owned(s.replace('_', ""))
				} else {
					Cow::Borrowed(s)
				};

				let num = match u128::from_str_radix(&s, radix) {
					Ok(num) => num,
					Err(err) => return Err(self.error(start_pos, err.to_string())),
				};

				let num = match (neg, num) {
					(true, 0x8000_0000_0000_0000_0000_0000_0000_0000) => {
						return Err(self.error(start_pos, "integer literal cannot fit in i128"));
					}
					(true, _) => -(num as i128),
					(false, _) => num as i128,
				};
				let tok = Token::Int(num);

				Ok(Some((start_pos, tok)))
			}
			c => Err(self.error(self.pos, format!("unexpected character '{c}'"))),
		}
	}

	#[allow(clippy::should_implement_trait)]
	pub fn next(&mut self) -> Result<Option<(Pos, Token)>> {
		let tok = self.lookahead.take();
		self.reload()?;
		Ok(tok)
	}

	#[must_use]
	pub const fn peek(&self) -> Option<&(Pos, Token)> {
		self.lookahead.as_ref()
	}

	#[must_use]
	pub const fn is_eof(&self) -> bool {
		self.lookahead.is_none()
	}

	fn reload(&mut self) -> Result {
		if self.lookahead.is_none() && self.pos.offset < self.source.len() {
			self.lookahead = self.next_token()?;
		}

		Ok(())
	}

	fn lookahead_byte(&self, n: usize) -> Option<u8> {
		self.source.as_bytes().get(self.pos.offset + n).copied()
	}

	fn peek_byte(&self) -> Option<u8> {
		self.lookahead_byte(0)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
	LParen,
	RParen,
	Symbol(String),
	Int(i128),
	At,
}

impl Token {
	#[must_use]
	pub const fn is_int(&self) -> bool {
		matches!(self, Self::Int(..))
	}

	#[must_use]
	pub const fn is_sym(&self) -> bool {
		matches!(self, Self::Symbol(..))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[track_caller]
	fn lex(src: &str) -> Vec<Token> {
		let mut tokens = Vec::new();
		let mut lexer = Lexer::new(0, src).unwrap();
		while let Some((_, token)) = lexer.next().unwrap() {
			tokens.push(token);
		}

		tokens
	}

	#[test]
	fn basic() {
		assert_eq!(
			lex(
				";; comment\n; another\r\n   \t(one two three (; block comment ;) 23 (; nested (; block ;) comment ;) -568  )\n"
			),
			[
				Token::LParen,
				Token::Symbol("one".to_owned()),
				Token::Symbol("two".to_owned()),
				Token::Symbol("three".to_owned()),
				Token::Int(23),
				Token::Int(-568),
				Token::RParen
			]
		);
	}

	#[test]
	fn ends_with_sym() {
		assert_eq!(lex("asdf"), [Token::Symbol("asdf".to_owned())]);
	}

	#[test]
	fn ends_with_num() {
		assert_eq!(lex("23"), [Token::Int(23)]);
	}

	#[test]
	fn weird_syms() {
		assert_eq!(
			lex("(+ [] => !! _test!;comment\n)"),
			[
				Token::LParen,
				Token::Symbol("+".to_owned()),
				Token::Symbol("[]".to_owned()),
				Token::Symbol("=>".to_owned()),
				Token::Symbol("!!".to_owned()),
				Token::Symbol("_test!".to_owned()),
				Token::RParen
			]
		);
	}

	#[test]
	fn integers() {
		assert_eq!(
			lex("0 1 -1"),
			[Token::Int(0), Token::Int(1), Token::Int(-1)]
		);

		assert_eq!(
			lex("340_282_366_920_938_463_463_374_607_431_768_211_455"),
			[Token::Int(-1)]
		);

		assert_eq!(
			lex("170_141_183_460_469_231_731_687_303_715_884_105_727"),
			[Token::Int(i128::MAX)]
		);

		assert!(Lexer::new(0, "-170_141_183_460_469_231_731_687_303_715_884_105_728").is_err());
	}
}
