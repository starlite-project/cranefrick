use serde::{Deserialize, Serialize};

use super::FactoredOffsetCellOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubOptions {
	CellAt(FactoredOffsetCellOptions<u8>),
	FromCell(FactoredOffsetCellOptions<u8>),
}
