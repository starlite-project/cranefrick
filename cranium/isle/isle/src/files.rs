use std::{
	error::Error,
	fmt::{Display, Formatter, Result as FmtResult},
	io::Error as IoError,
	ops::Index,
	path::{Path, PathBuf},
	slice::SliceIndex,
};

use super::codegen::Prefix;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Files {
	pub names: Vec<String>,
	pub texts: Vec<String>,
	line_maps: Vec<LineMap>,
}

impl Files {
	pub fn from_paths<P>(
		paths: impl IntoIterator<Item = P>,
		prefixes: &[Prefix],
	) -> Result<Self, FromFilesError>
	where
		P: AsRef<Path>,
	{
		fn replace_prefixes(prefixes: &[Prefix], path: String) -> String {
			for Prefix { prefix, name } in prefixes {
				if path.starts_with(prefix) {
					return path.replacen(prefix, name, 1);
				}
			}

			path
		}

		let mut names = Vec::new();
		let mut texts = Vec::new();
		let mut line_maps = Vec::new();

		for path in paths {
			let path = path.as_ref();
			let contents = std::fs::read_to_string(path)
				.map_err(|e| FromFilesError::new(path.to_owned(), e))?;
			let name = replace_prefixes(prefixes, path.display().to_string());

			line_maps.push(LineMap::from_str(&contents));
			names.push(name);
			texts.push(contents);
		}

		Ok(Self {
			names,
			texts,
			line_maps,
		})
	}

	pub fn from_names_and_contents(files: impl IntoIterator<Item = (String, String)>) -> Self {
		let mut names = Vec::new();
		let mut texts = Vec::new();
		let mut line_maps = Vec::new();

		for (name, contents) in files {
			line_maps.push(LineMap::from_str(&contents));
			names.push(name);
			texts.push(contents);
		}

		Self {
			names,
			texts,
			line_maps,
		}
	}

	pub fn file_name(&self, file: usize) -> Option<&str> {
		self.names.get(file).map(String::as_str)
	}

	pub fn file_text(&self, file: usize) -> Option<&str> {
		self.texts.get(file).map(String::as_str)
	}

	#[must_use]
	pub fn file_line_map(&self, file: usize) -> Option<&LineMap> {
		self.line_maps.get(file)
	}
}

#[derive(Debug)]
pub struct FromFilesError(pub PathBuf, pub IoError);

impl FromFilesError {
	const fn new(path: PathBuf, source: IoError) -> Self {
		Self(path, source)
	}
}

impl Display for FromFilesError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("failed to read the file at path ")?;
		Display::fmt(&self.0.display(), f)
	}
}

impl Error for FromFilesError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		Some(&self.1)
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct LineMap {
	line_ends: Vec<usize>,
}

impl LineMap {
	#[allow(clippy::should_implement_trait)]
	#[must_use]
	pub fn from_str(text: &str) -> Self {
		let line_ends = text.match_indices('\n').map(|(i, ..)| i + 1).collect();
		Self { line_ends }
	}

	#[must_use]
	pub fn line(&self, pos: usize) -> usize {
		self.line_ends.partition_point(|&end| end <= pos)
	}

	#[must_use]
	pub fn get(&self, line: usize) -> Option<&usize> {
		self.line_ends.get(line)
	}
}

impl<I> Index<I> for LineMap
where
	I: SliceIndex<[usize]>,
{
	type Output = I::Output;

	fn index(&self, index: I) -> &Self::Output {
		self.line_ends.index(index)
	}
}
