#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

mod error;

use std::{
	borrow::Cow,
	cmp,
	collections::{BTreeMap, BTreeSet},
	fmt::{Display, Formatter as FmtFormatter, Result as FmtResult, Write as _},
	fs,
	io::Write,
	path::Path,
};

pub use self::error::*;

static SHIFTWIDTH: usize = 4;

#[macro_export]
macro_rules! loc {
	() => {
		$crate::FileLocation::new(file!(), line!())
	};
}

#[macro_export]
macro_rules! fmtln {
    ($fmt:ident, $fmtstring:expr, $($fmtargs:expr),*) => {
        $fmt.line_with_location(format!($fmtstring, $($fmtargs),*), $crate::loc!())
    };

    ($fmt:ident, $arg:expr) => {
        $fmt.line_with_location(format!($arg), $crate::loc!())
    };

    ($_:tt, $($args:expr),+) => {
        compile_error!("This macro requires at least two arguments: the Formatter instance and a format string.")
    };

    ($_:tt) => {
        compile_error!("This macro requires at least two arguments: the Formatter instance and a format string.")
    };
}

#[derive(Debug, Clone, Copy)]
pub struct FileLocation {
	file: &'static str,
	line: u32,
}

impl FileLocation {
	#[must_use]
	pub const fn new(file: &'static str, line: u32) -> Self {
		Self { file, line }
	}
}

impl Display for FileLocation {
	fn fmt(&self, f: &mut FmtFormatter<'_>) -> FmtResult {
		f.write_str(self.file)?;
		f.write_char(':')?;
		Display::fmt(&self.line, f)
	}
}

#[derive(Debug)]
pub struct Formatter {
	indent: usize,
	lines: Vec<String>,
	lang: Language,
}

impl Formatter {
	#[must_use]
	pub const fn new(lang: Language) -> Self {
		Self {
			indent: 0,
			lines: Vec::new(),
			lang,
		}
	}

	pub const fn push_indent(&mut self) {
		self.indent += 1;
	}

	pub fn pop_indent(&mut self) {
		assert!(self.indent > 0, "already at top level indentation");
		self.indent -= 1;
	}

	pub fn indent<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
		self.push_indent();
		let ret = f(self);
		self.pop_indent();
		ret
	}

	fn get_indent(&self) -> Cow<'static, str> {
		match self.indent {
			0 => Cow::Borrowed(""),
			x => Cow::Owned(format!("{:-1$}", " ", x * SHIFTWIDTH)),
		}
	}

	pub fn line(&mut self, contents: impl Display) {
		let indented_line = format!("{}{contents}", self.get_indent());
		self.lines.push(indented_line);
	}

	pub fn line_with_location(&mut self, contents: impl Display, location: FileLocation) {
		let indent = self.get_indent();
		let contents = contents.to_string();
		let indented_line = if self.lang.should_append_location(&contents) {
			let comment_token = self.lang.comment_token();
			format!("{indent}{contents} {comment_token} {location}\n")
		} else {
			format!("{indent}{contents}\n")
		};

		self.lines.push(indented_line);
	}

	pub fn empty_line(&mut self) {
		self.lines.push('\n'.to_string());
	}

	pub fn multi_line(&mut self, s: &str) {
		parse_multiline(s).into_iter().for_each(|l| self.line(&l));
	}

	pub fn comment(&mut self, s: impl Display) {
		self.line(format_args!("{} {s}", self.lang.comment_token()));
	}

	pub fn doc_comment(&mut self, contents: impl Display) {
		assert!(matches!(self.lang, Language::Rust));
		parse_multiline(&contents.to_string())
			.iter()
			.map(|l| {
				if l.is_empty() {
					"///".to_owned()
				} else {
					format!("/// {l}")
				}
			})
			.for_each(|l| self.line(l));
	}

	pub fn add_block<T>(&mut self, start: &str, f: impl FnOnce(&mut Self) -> T) -> T {
		assert!(matches!(self.lang, Language::Rust));
		self.line(format_args!("{start} {{"));
		let ret = self.indent(f);
		self.line('}');
		ret
	}

	pub fn add_match(&mut self, m: Match) {
		assert!(matches!(self.lang, Language::Rust));
		fmtln!(self, "match {} {{", m.expr);
		self.indent(|fmt| {
			for ((fields, body), names) in &m.arms {
				let conditions = names
					.iter()
					.map(|name| {
						if fields.is_empty() {
							name.clone()
						} else {
							format!("{} {{ {} }}", name, fields.join(", "))
						}
					})
					.collect::<Vec<_>>()
					.join(" |\n") + " => {";

				fmt.multi_line(&conditions);
				fmt.indent(|fmt| {
					fmt.line(body);
				});

				fmt.line('}');
			}

			if let Some(body) = m.catch_all {
				fmt.line("_ => {");
				fmt.indent(|fmt| fmt.line(body));
				fmt.line('}');
			}
		});

		self.line('}');
	}

	pub fn write(&self, filename: impl AsRef<Path>, directory: &Path) -> Result<(), Error> {
		let path = directory.join(&filename);
		eprintln!("writing generated file {}", path.display());
		let mut f = fs::File::create(path)?;

		for l in self.lines.iter().map(String::as_bytes) {
			f.write_all(l)?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct Match {
	expr: String,
	arms: BTreeMap<(Vec<String>, String), BTreeSet<String>>,
	catch_all: Option<String>,
}

impl Match {
	pub fn new(expr: impl Into<String>) -> Self {
		Self {
			expr: expr.into(),
			arms: BTreeMap::new(),
			catch_all: None,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
	Rust,
	Isle,
}

impl Language {
	#[must_use]
	pub fn should_append_location(self, line: &str) -> bool {
		match self {
			Self::Isle => true,
			Self::Rust => !line.ends_with(['{', '}']),
		}
	}

	#[must_use]
	pub const fn comment_token(self) -> &'static str {
		match self {
			Self::Rust => "//",
			Self::Isle => ";;",
		}
	}
}

fn indent(s: &str) -> Option<usize> {
	if s.is_empty() {
		None
	} else {
		let t = s.trim_start();
		Some(s.len() - t.len())
	}
}

fn parse_multiline(s: &str) -> Vec<String> {
	let expanded_tab = format!("{:-1$}", " ", SHIFTWIDTH);
	let lines = s
		.lines()
		.map(|l| l.replace('\t', &expanded_tab))
		.collect::<Vec<_>>();

	let indent = lines
		.iter()
		.skip(1)
		.filter(|l| !l.trim().is_empty())
		.map(|l| l.len() - l.trim_start().len())
		.min();

	let mut lines_iter = lines.iter().skip_while(|l| l.is_empty());
	let mut trimmed = Vec::with_capacity(lines.len());

	if let Some(s) = lines_iter.next().map(|l| l.trim()).map(ToString::to_string) {
		trimmed.push(s);
	}

	let mut other_lines = if let Some(indent) = indent {
		lines_iter
			.map(|l| &l[cmp::min(indent, l.len())..])
			.map(str::trim_end)
			.map(ToString::to_string)
			.collect::<Vec<_>>()
	} else {
		lines_iter
			.map(|l| l.trim_end())
			.map(ToString::to_string)
			.collect()
	};

	trimmed.append(&mut other_lines);

	while let Some(s) = trimmed.pop() {
		if s.is_empty() {
			continue;
		}

		trimmed.push(s);
		break;
	}

	trimmed
}
