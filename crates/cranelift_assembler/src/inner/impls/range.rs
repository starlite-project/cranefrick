use std::ops::RangeInclusive;

use crate::inner::InnerAssembler;

impl InnerAssembler<'_> {
	pub fn set_range(&mut self, value: u8, range: RangeInclusive<i32>) {
		for i in range {
			self.set_cell(value, i);
		}
	}
}
