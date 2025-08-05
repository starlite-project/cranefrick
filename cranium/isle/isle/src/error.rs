use std::{
	error::Error as StdError,
	fmt::{Debug, Display, Formatter, Result as FmtResult},
	io::Error as IoError,
	sync::Arc,
};

use codespan_reporting::{
	diagnostic::{Diagnostic, Label},
	files::SimpleFiles,
	term::{Config, termcolor},
};
use cranefrick_utils::IntoIteratorExt as _;

use super::{files::Files, lexer::Pos};

#[derive(Debug, Clone, Copy)]
pub struct Span {
	pub from: Pos,
	pub to: Pos,
}

impl Span {
	#[must_use]
	pub const fn from_single(pos: Pos) -> Self {
		Self {
			from: pos,
			to: Pos {
				file: pos.file,
				offset: pos.offset + 1,
			},
		}
	}
}

impl From<Pos> for Span {
	fn from(value: Pos) -> Self {
		Self::from_single(value)
	}
}

impl From<&Span> for std::ops::Range<usize> {
	fn from(value: &Span) -> Self {
		value.from.offset..value.to.offset
	}
}

pub struct Errors {
	pub errors: Vec<Error>,
	pub(crate) files: Arc<Files>,
}

impl Errors {
	pub fn new(errors: impl IntoIterator<Item = Error>, files: Arc<Files>) -> Self {
		Self {
			errors: errors.collect_to(),
			files,
		}
	}

	pub fn from_io(error: IoError, context: impl Into<String>) -> Self {
		Self::new(
			[Error::Io {
				source: error,
				context: context.into(),
			}],
			Arc::new(Files::default()),
		)
	}

	fn emit(&self, f: &mut Formatter<'_>, diagnostics: Vec<Diagnostic<usize>>) -> FmtResult {
		let w = termcolor::BufferWriter::stderr(termcolor::ColorChoice::Auto);
		let mut b = w.buffer();
		let mut files = SimpleFiles::new();
		for (name, source) in self.files.names.iter().zip(self.files.texts.iter()) {
			files.add(name, source);
		}

		for diagnostic in diagnostics {
			codespan_reporting::term::emit(&mut b, &Config::default(), &files, &diagnostic)
				.map_err(|_| std::fmt::Error)?;
		}

		let b = b.into_inner();
		let b = std::str::from_utf8(&b).map_err(|_| std::fmt::Error)?;
		f.write_str(b)
	}
}

impl Debug for Errors {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		if self.errors.is_empty() {
			return Ok(());
		}

		let diagnostics = self
			.errors
			.iter()
			.map(|e| {
				let message = e.to_string();

				let labels = match e {
					Error::Io { .. } => Vec::new(),
					Error::Parse { span, .. }
					| Error::Type { span, .. }
					| Error::Unreachable { span, .. } => vec![Label::primary(span.from.file, span)],
					Error::Overlap { rules, .. } => {
						let mut labels = vec![Label::primary(rules[0].from.file, &rules[0])];

						labels.extend(
							rules[1..]
								.iter()
								.map(|span| Label::secondary(span.from.file, span)),
						);

						labels
					}
					Error::Shadowed { shadowed, mask } => {
						let mut labels = vec![Label::primary(mask.from.file, mask)];
						labels.extend(
							shadowed
								.iter()
								.map(|span| Label::secondary(span.from.file, span)),
						);
						labels
					}
				};

				let mut sources = Vec::new();
				let mut source = e.source();
				while let Some(e) = source {
					sources.push(format!("{e:?}"));
					source = StdError::source(e);
				}

				Diagnostic::error()
					.with_message(message)
					.with_labels(labels)
					.with_notes(sources)
			})
			.collect::<Vec<_>>();
		self.emit(f, diagnostics)?;

		if self.errors.len() > 1 {
			f.write_str("found ")?;
			Display::fmt(&self.errors.len(), f)?;
			f.write_str(" errors\n")?;
		}

		Ok(())
	}
}

#[derive(Debug)]
pub enum Error {
	Io { source: IoError, context: String },
	Parse { message: String, span: Span },
	Type { message: String, span: Span },
	Unreachable { message: String, span: Span },
	Overlap { message: String, rules: Vec<Span> },
	Shadowed { shadowed: Vec<Span>, mask: Span },
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Io { context, .. } => f.write_str(context),
			Self::Parse { message, .. } => {
				f.write_str("parse error: ")?;
				f.write_str(message)
			}
			Self::Type { message, .. } => {
				f.write_str("type error: ")?;
				f.write_str(message)
			}
			Self::Unreachable { message, .. } => {
				f.write_str("unreachable rule: ")?;
				f.write_str(message)
			}
			Self::Overlap { message, .. } => {
				f.write_str("overlap error: ")?;
				f.write_str(message)
			}
			Self::Shadowed { .. } => {
				f.write_str("more general higher-priority rule shadows other rules")
			}
		}
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::Io { source, .. } => Some(source),
			_ => None,
		}
	}
}
