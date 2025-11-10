mod cell;
mod intrinsics;
mod io;
mod loops;
mod mem;
mod metadata;
mod pointer;
mod value;

use frick_utils::Convert as _;
use inkwell::values::IntValue;

use super::InnerAssembler;
use crate::{AssemblyError, ContextGetter as _};

impl<'ctx> InnerAssembler<'ctx> {
	fn resolve_factor(
		&self,
		lhs: IntValue<'ctx>,
		rhs_imm: u8,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let i8_type = self.context().i8_type();

		Ok(if rhs_imm.is_power_of_two() {
			let rhs = i8_type.const_int(rhs_imm.ilog2().convert::<u64>(), false);

			self.builder.build_left_shift(lhs, rhs, "\0")
		} else {
			let rhs = i8_type.const_int(rhs_imm.convert::<u64>(), false);

			self.builder.build_int_mul(lhs, rhs, "\0")
		}?)
	}
}
