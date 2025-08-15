use cranelift_codegen::ir::{types, InstBuilder as _};

use crate::assembler::Assembler;

impl Assembler<'_> {
    pub fn change_cell(&mut self, value: i8, offset: i32) {
        self.invalidate_load();

        let heap_value = self.load(offset);
        let changed = if value.is_negative() {
            let sub_value = self.ins().iconst(types::I8, i64::from(value.unsigned_abs()));
            self.ins().isub(heap_value, sub_value)
        } else {
            self.ins().iadd_imm(heap_value, i64::from(value))
        };

        self.store(changed, offset);
    }

    pub fn set_cell(&mut self, value: u8, offset: i32) {
        self.invalidate_load();

        let value = self.const_u8(value);
        self.store(value, offset);
    }
}
