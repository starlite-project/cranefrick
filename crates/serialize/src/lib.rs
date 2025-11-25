#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{fs, io::Error as IoError, path::Path};

use serde::Serialize;

#[derive(Debug)]
pub enum SerializeError {
	#[cfg(feature = "ron")]
	Ron(ron::Error),
	Io(IoError),
}

impl From<IoError> for SerializeError {
	fn from(value: IoError) -> Self {
		Self::Io(value)
	}
}

#[cfg(feature = "ron")]
impl From<ron::Error> for SerializeError {
	fn from(value: ron::Error) -> Self {
		Self::Ron(value)
	}
}

pub fn serialize<T: Serialize>(
	value: &T,
	folder_path: &Path,
	file_name: &str,
) -> Result<(), SerializeError> {
	#[cfg(feature = "ron")]
	serialize_as_ron(value, folder_path, file_name)?;

	Ok(())
}

#[cfg(feature = "ron")]
fn serialize_as_ron<T: Serialize>(
	value: &T,
	folder_path: &Path,
	file_name: &str,
) -> Result<(), SerializeError> {
	let mut output = String::new();
	let mut serializer = ron::Serializer::with_options(
		&mut output,
		Some(ron::ser::PrettyConfig::new().separate_tuple_members(true)),
		&ron::Options::default().without_recursion_limit(),
	)?;

	value.serialize(&mut serializer)?;

	drop(serializer);

	fs::write(folder_path.join(format!("{file_name}.ron")), output)?;

	Ok(())
}
