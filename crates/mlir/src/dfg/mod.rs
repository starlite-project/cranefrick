use core::num::NonZeroI32;

use serde::{Deserialize, Serialize};

pub enum Node {
	ChangeCell(i8, Option<NonZeroI32>),
}
