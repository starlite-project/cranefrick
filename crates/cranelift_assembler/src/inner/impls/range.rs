use std::ops::RangeInclusive;

use cranelift_codegen::ir::InstBuilder as _;
use crate::inner::{InnerAssembler, SrcLoc};

impl InnerAssembler<'_> {
    pub fn set_range(&mut self, value: u8, range: RangeInclusive<i32>) {
        self.add_srcflag(SrcLoc::SET_RANGE);

        for i in range {
            self.set_cell(value, i);
        }
    }
}
