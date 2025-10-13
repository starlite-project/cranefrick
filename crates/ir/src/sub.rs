use serde::{Deserialize, Serialize};

use super::ChangeCellOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubType {
	Value(u8),
	CellAt(ChangeCellOptions),
	FromCell(ChangeCellOptions),
}
