use alloc::vec::Vec;
use core::fmt::{Display, Error as FmtError, Formatter, Result as FmtResult, Write as _};

use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

use super::ValuedOffsetCellOptions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OutputOptions {
	Cell(ValuedOffsetCellOptions<i8>),
	Cells(Vec<ValuedOffsetCellOptions<i8>>),
	Char(u8),
	Str(Vec<u8>),
}

impl OutputOptions {
	#[must_use]
	pub const fn cell(value_offset: i8, offset: i32) -> Self {
		Self::Cell(ValuedOffsetCellOptions::new(value_offset, offset))
	}

	#[must_use]
	pub fn cells(values: impl IntoIterator<Item = ValuedOffsetCellOptions<i8>>) -> Self {
		Self::Cells(values.collect_to())
	}

	#[must_use]
	pub const fn char(value: u8) -> Self {
		Self::Char(value)
	}

	#[must_use]
	pub fn str(values: impl IntoIterator<Item = u8>) -> Self {
		Self::Str(values.collect_to())
	}
}

impl Display for OutputOptions {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("output_")?;

		match self {
			Self::Cell(output_options) => {
				f.write_str("cell")?;
				match output_options.into_parts() {
					(0, 0) => {}
					(a, 0) => {
						f.write_char('(')?;
						Display::fmt(&a, f)?;
						f.write_char(')')?;
					}
					(0, x) => {
						f.write_str("_at(")?;
						Display::fmt(&x, f)?;
						f.write_char(')')?;
					}
					(a, x) => {
						f.write_str("_at(")?;
						Display::fmt(&a, f)?;
						f.write_str(", ")?;
						Display::fmt(&x, f)?;
						f.write_char(')')?;
					}
				}
			}
			Self::Cells(output_options) => {
				f.write_str("cells(")?;

				let mut is_first = true;
				for (a, x) in output_options.iter().map(|x| x.into_parts()) {
					if !is_first {
						f.write_str(", ")?;
					}

					f.write_char('(')?;
					Display::fmt(&a, f)?;
					f.write_str(", ")?;
					Display::fmt(&x, f)?;
					f.write_char(')')?;

					is_first = false;
				}
			}
			&Self::Char(c) => {
				f.write_str("char(")?;
				Display::fmt(&(c as char).escape_debug(), f)?;
				f.write_char(')')?;
			}
			Self::Str(chars) => {
				f.write_str("str(")?;
				for char in chars.iter().copied().map(|c| c as char) {
					Display::fmt(&char.escape_debug(), f)?;
				}

				f.write_char(')')?;
			}
			_ => return Err(FmtError),
		}

		Ok(())
	}
}
