use std::ops::RangeInclusive;

use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
	pub fn mem_set(&mut self, value: u8, range: RangeInclusive<i32>) {
		self.add_srcflag(SrcLoc::SET_RANGE);

		for i in range {
			self.set_cell(value, i);
		}

		self.remove_srcflag(SrcLoc::SET_RANGE);
	}

	pub fn change_range(&mut self, value: i8, range: RangeInclusive<i32>) {
		self.add_srcflag(SrcLoc::CHANGE_RANGE);

		for i in range {
			self.change_cell(value, i);
		}

		self.remove_srcflag(SrcLoc::CHANGE_RANGE);
	}
}
