use alloc::{borrow::ToOwned, string::ToString as _, vec::Vec};
use core::ops::Range;
use std::{
	fs::{self, File},
	io::{self, BufReader, Read, Seek},
	path::{Path, PathBuf},
};

use ariadne::{FileCache, IndexType, Label, Report, ReportKind};
use chumsky::{input::ValueInput, prelude::*};

use crate::{BrainOperation, BrainOperationType};

pub fn parse(file_path: impl AsRef<Path>) -> io::Result<Vec<BrainOperation>> {
	let file_path = file_path.as_ref().to_owned();

	let file = fs::File::open(&file_path)?;

	match parser().parse(CharIoInput::new(file)).into_result() {
		Ok(e) => Ok(e),
		Err(errs) => {
			for err in errs {
				let report = Report::build(
					ReportKind::Error,
					ErrorSpan {
						file_path: file_path.clone(),
						span: err.span().into_range(),
					},
				)
				.with_config(ariadne::Config::new().with_index_type(IndexType::Byte))
				.with_message(err.to_string())
				.with_label(Label::new(ErrorSpan {
					file_path: file_path.clone(),
					span: err.span().into_range(),
				}))
				.finish();
				let cache = FileCache::default();

				if let Some(indicatif_writer) =
					tracing_indicatif::writer::get_indicatif_stderr_writer()
				{
					report.write(cache, indicatif_writer)?;
				} else {
					report.eprint(cache)?;
				}
			}

			Ok(Vec::new())
		}
	}
}

fn parser<'src>()
-> impl Parser<'src, CharIoInput<File>, Vec<BrainOperation>, extra::Err<Rich<'src, char>>> {
	recursive(|expr| {
		choice((
			just('+').to(BrainOperationType::ChangeCell(1)),
			just('-').to(BrainOperationType::ChangeCell(-1)),
			just('<').to(BrainOperationType::MovePointer(-1)),
			just('>').to(BrainOperationType::MovePointer(1)),
			just('.').to(BrainOperationType::OutputCurrentCell),
			just(',').to(BrainOperationType::InputIntoCell),
			none_of("+-<>.,[]").map(BrainOperationType::Comment),
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

struct ErrorSpan {
	file_path: PathBuf,
	span: Range<usize>,
}

impl ariadne::Span for ErrorSpan {
	type SourceId = Path;

	fn source(&self) -> &Self::SourceId {
		self.file_path.as_path()
	}

	fn start(&self) -> usize {
		self.span.start
	}

	fn end(&self) -> usize {
		self.span.end
	}
}

struct CharIoInput<R> {
	reader: BufReader<R>,
	last_cursor: usize,
}

impl<R> CharIoInput<R>
where
	R: Read + Seek,
{
	pub fn new(reader: R) -> Self {
		Self {
			reader: BufReader::new(reader),
			last_cursor: 0,
		}
	}
}

impl<'src, R> Input<'src> for CharIoInput<R>
where
	R: Read + Seek + 'src,
{
	type Cache = Self;
	type Cursor = usize;
	type MaybeToken = char;
	type Span = SimpleSpan;
	type Token = char;

	fn begin(self) -> (Self::Cursor, Self::Cache) {
		(0, self)
	}

	fn cursor_location(cursor: &Self::Cursor) -> usize {
		*cursor
	}

	unsafe fn next_maybe(
		this: &mut Self::Cache,
		cursor: &mut Self::Cursor,
	) -> Option<Self::MaybeToken> {
		unsafe { Self::next(this, cursor) }
	}

	unsafe fn span(_: &mut Self::Cache, range: Range<&Self::Cursor>) -> Self::Span {
		(*range.start..*range.end).into()
	}
}

impl<'src, R> ValueInput<'src> for CharIoInput<R>
where
	R: Read + Seek + 'src,
{
	unsafe fn next(this: &mut Self::Cache, cursor: &mut Self::Cursor) -> Option<Self::Token> {
		if *cursor != this.last_cursor {
			let seek = *cursor as i64 - this.last_cursor as i64;

			this.reader.seek_relative(seek).unwrap();

			this.last_cursor = *cursor;
		}

		let mut out = 0;
		let r = this.reader.read_exact(std::slice::from_mut(&mut out));

		match r {
			Ok(()) => {
				this.last_cursor += 1;
				*cursor += 1;
				Some(out as char)
			}
			Err(..) => None,
		}
	}
}
