mod cell;
mod io;
mod loops;
mod mem;
mod pointer;
mod value;

use inkwell::{
	IntPredicate,
	builder::Builder,
	values::{FunctionValue, IntValue, PointerValue},
};

use crate::{AssemblyError, ContextExt as _, ContextGetter as _, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip_all)]
	fn start_lifetime(
		&self,
		alloc_len: IntValue<'ctx>,
		pointer: PointerValue<'ctx>,
	) -> Result<impl Drop, AssemblyError> {
		struct LifetimeEnd<'builder, 'ctx> {
			builder: &'builder Builder<'ctx>,
			end: FunctionValue<'ctx>,
			pointer: PointerValue<'ctx>,
			alloc_len: IntValue<'ctx>,
		}

		impl Drop for LifetimeEnd<'_, '_> {
			fn drop(&mut self) {
				self.builder
					.build_call(
						self.end,
						&[self.alloc_len.into(), self.pointer.into()],
						"\0",
					)
					.unwrap();
			}
		}

		self.builder.build_call(
			self.functions.lifetime.start,
			&[alloc_len.into(), pointer.into()],
			"\0",
		)?;

		let lifetime_end = LifetimeEnd {
			builder: &self.builder,
			end: self.functions.lifetime.end,
			pointer,
			alloc_len,
		};

		Ok(lifetime_end)
	}

	#[tracing::instrument(skip_all)]
	pub fn write_puts(&self) -> Result<(), AssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();
		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_type = context.default_ptr_type();

		self.builder.unset_current_debug_location();

		let entry_block = context.append_basic_block(self.functions.puts, "entry\0");
		let try_block = context.append_basic_block(self.functions.puts, "try\0");
		let continue_block = context.append_basic_block(self.functions.puts, "continue\0");
		let catch_block = context.append_basic_block(self.functions.puts, "catch\0");
		let exit_block = context.append_basic_block(self.functions.puts, "exit\0");

		self.builder.position_at_end(entry_block);

		let (ptr_param, string_len) = {
			let params = self.functions.puts.get_params();

			(params[0].into_pointer_value(), params[1].into_int_value())
		};

		let assert_ptr_not_null = self.builder.build_is_not_null(ptr_param, "\0")?;

		self.builder.build_direct_call(
			self.functions.assume,
			&[assert_ptr_not_null.into()],
			"\0",
		)?;

		let i64_zero = i64_type.const_zero();

		let is_str_empty =
			self.builder
				.build_int_compare(IntPredicate::EQ, i64_zero, string_len, "\0")?;

		self.builder
			.build_conditional_branch(is_str_empty, exit_block, try_block)?;

		self.builder.position_at_end(try_block);

		let idx_phi_value = self.builder.build_phi(i64_type, "\0")?;

		let char_ptr = self.gep(
			i8_type,
			ptr_param,
			idx_phi_value.as_basic_value().into_int_value(),
			"\0",
		)?;

		let raw_char = self
			.builder
			.build_load(i8_type, char_ptr, "\0")?
			.into_int_value();

		let extended_char = self
			.builder
			.build_int_z_extend_or_bit_cast(raw_char, i32_type, "\0")?;

		self.builder.build_direct_invoke(
			self.functions.putchar,
			&[extended_char.into()],
			continue_block,
			catch_block,
			"\0",
		)?;

		self.builder.position_at_end(continue_block);

		let i64_one = i64_type.const_int(1, false);

		let next_index = self.builder.build_int_add(
			idx_phi_value.as_basic_value().into_int_value(),
			i64_one,
			"\0",
		)?;

		idx_phi_value.add_incoming(&[(&i64_zero, entry_block), (&next_index, continue_block)]);

		let is_done =
			self.builder
				.build_int_compare(IntPredicate::EQ, next_index, string_len, "\0")?;

		self.builder
			.build_conditional_branch(is_done, exit_block, try_block)?;

		self.builder.position_at_end(catch_block);
		let exception_type = context.struct_type(&[ptr_type.into(), i32_type.into()], false);

		let exception = self.builder.build_landing_pad(
			exception_type,
			self.functions.eh_personality,
			&[],
			true,
			"\0",
		)?;

		self.builder.build_resume(exception)?;

		self.builder.position_at_end(exit_block);
		self.builder.build_return(None)?;

		Ok(())
	}
}

fn create_string(prefix: &str, suffix: &'static str) -> String {
	let mut out = String::with_capacity(prefix.len() + suffix.len());

	out.push_str(prefix);
	out.push_str(suffix);

	out
}
