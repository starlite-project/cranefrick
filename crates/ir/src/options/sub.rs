use serde::{Deserialize, Serialize};

use super::FactoredChangeCellOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubOptions {
	CellAt(FactoredChangeCellOptions<u8>),
	FromCell(FactoredChangeCellOptions<u8>),
}
