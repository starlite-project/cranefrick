use serde::{Deserialize, Serialize};

use super::ChangeCellOptions;
use crate::Factor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubType {
	Value(u8),
	CellAt(ChangeCellOptions<u8, Factor>),
	FromCell(ChangeCellOptions<u8, Factor>),
}
