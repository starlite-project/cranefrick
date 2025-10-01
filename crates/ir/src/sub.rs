use serde::{Deserialize, Serialize};

use super::CellChangeOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubType {
	Value(u8),
	CellAt(CellChangeOptions),
	FromCell(CellChangeOptions),
}
