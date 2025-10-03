use std::{fmt::Display, slice};

use frick_assembler::AssemblyError;
use frick_ir::{BrainIr, CellChangeOptions, OutputOptions};
use inkwell::{
	attributes::{Attribute, AttributeLoc},
	context::ContextRef,
	types::{ArrayType, IntType},
	values::{CallSiteValue, InstructionValueError, IntValue, PointerValue},
};

use crate::{LlvmAssemblyError, inner::InnerAssembler};

impl<'ctx> InnerAssembler<'ctx> {
	pub fn output(&self, options: &OutputOptions) -> Result<(), AssemblyError<LlvmAssemblyError>> {
		match options {
			OutputOptions::Cell(options) => {
				self.output_cells_puts(slice::from_ref(options))?;
			}
			OutputOptions::Cells(options) => self.output_cells(options)?,
			OutputOptions::Char(c) => self.output_chars(slice::from_ref(c))?,
			OutputOptions::Str(c) => self.output_chars(c)?,
			_ => {
				return Err(AssemblyError::NotImplemented(BrainIr::Output(
					options.clone(),
				)));
			}
		}

		Ok(())
	}

	fn output_cells(&self, options: &[CellChangeOptions<i8>]) -> Result<(), LlvmAssemblyError> {
		if options.len() < 8 {
			self.output_cells_puts(options)
		} else {
			self.output_cells_iterated(options)
		}
	}

	fn output_cells_puts(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		assert!(options.len() < 8);

		let context = self.context();

		let i8_type = context.i8_type();
		let i64_type = context.i64_type();

		let array_len = i64_type.const_int(options.len() as u64, false);

		let _lifetime = {
			let lifetime_array_len = i64_type.const_int(options.len() as u64 + 1, false);

			self.start_lifetime(lifetime_array_len, self.pointers.output)?
		};

		if options.windows(2).all(|w| w[0] == w[1]) {
			self.setup_output_cells_puts_memset(i8_type, i64_type, options[0], options.len() as u64)
		} else {
			self.setup_output_cells_puts_iterated(i8_type, options)
		}?;

		let zero = i8_type.const_zero();

		let last_index_gep = unsafe {
			self.builder.build_in_bounds_gep(
				i8_type,
				self.pointers.output,
				&[array_len],
				"output_cells_puts_gep",
			)?
		};

		self.builder.build_store(last_index_gep, zero)?;

		let puts_call = self.builder.build_call(
			self.functions.puts,
			&[self.pointers.output.into(), array_len.into()],
			"output_cells_puts_call",
		)?;

		self.add_puts_io_attributes(
			puts_call,
			i8_type.array_type(options.len() as u32 + 1),
			options.len() as u64,
		);

		Ok(())
	}

	fn setup_output_cells_puts_memset(
		&self,
		i8_type: IntType<'ctx>,
		i64_type: IntType<'ctx>,
		options: CellChangeOptions<i8>,
		length: u64,
	) -> Result<(), LlvmAssemblyError> {
		let current_value = self.load(options.offset(), "setup_output_cells_puts_memset")?;

		let value_to_memset = if matches!(options.value(), 0) {
			current_value
		} else {
			let offset = i8_type.const_int(options.value() as u64, false);

			self.builder.build_int_add(
				current_value,
				offset,
				"setup_output_cells_puts_memset_add",
			)?
		};

		let array_len = i64_type.const_int(length, false);

		self.builder
			.build_memset(self.pointers.output, 1, value_to_memset, array_len)?;

		Ok(())
	}

	fn setup_output_cells_puts_iterated(
		&self,
		i8_type: IntType<'ctx>,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		let ptr_int_type = self.ptr_int_type;

		for (i, char) in options.iter().copied().enumerate() {
			let loaded_char = self.load(char.offset(), "setup_output_cells_puts_iterated")?;

			let offset_char = if matches!(char.value(), 0) {
				loaded_char
			} else {
				let offset_value = i8_type.const_int(char.value() as u64, false);

				self.builder.build_int_add(
					loaded_char,
					offset_value,
					"setup_output_cells_puts_iterated_add",
				)?
			};

			let array_offset = ptr_int_type.const_int(i as u64, false);

			let output_array_gep = unsafe {
				self.builder.build_in_bounds_gep(
					i8_type,
					self.pointers.output,
					&[array_offset],
					"setup_output_cells_puts_iterated_gep",
				)?
			};

			self.store_into(offset_char, output_array_gep)?;
		}

		Ok(())
	}

	fn output_cells_iterated(
		&self,
		options: &[CellChangeOptions<i8>],
	) -> Result<(), LlvmAssemblyError> {
		options
			.iter()
			.try_for_each(|x| self.output_cells(slice::from_ref(x)))
	}

	fn output_chars(&self, c: &[u8]) -> Result<(), LlvmAssemblyError> {
		let context = self.context();

		let i64_type = context.i64_type();

		let constant_initializer = context.const_string(c, true);

		let constant_string_ty = constant_initializer.get_type();

		let global_constant =
			self.module
				.add_global(constant_string_ty, None, "output_chars_global_value");

		self.setup_global_value(global_constant, &constant_initializer);

		let global_constant_pointer = global_constant.as_pointer_value();

		let array_len = i64_type.const_int(c.len() as u64, false);

		let _lifetime = {
			let lifetime_len = i64_type.const_int(c.len() as u64 + 1, false);

			self.start_lifetime(lifetime_len, global_constant_pointer)?
		};

		let puts_call = self.call_puts(global_constant_pointer, array_len, "output_chars")?;

		self.add_puts_io_attributes(puts_call, constant_string_ty, c.len() as u64);

		let puts_value = puts_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		let last = c.last().copied().unwrap();

		self.add_range_io_metadata(context, puts_value, last.into(), last.into())?;

		Ok(())
	}

	fn add_range_io_metadata(
		&self,
		context: ContextRef<'ctx>,
		char_call: IntValue<'ctx>,
		min_inclusive: u64,
		max_inclusive: u64,
	) -> Result<(), InstructionValueError> {
		let Some(char_instr) = char_call.as_instruction() else {
			return Ok(());
		};

		let i32_type = context.i32_type();

		let i32_i8_min = i32_type.const_int(min_inclusive, false);

		let i32_i8_max = i32_type.const_int(max_inclusive + 1, false);

		let range_metadata_id = context.get_kind_id("range");

		let range_metadata_node = self
			.context()
			.metadata_node(&[i32_i8_min.into(), i32_i8_max.into()]);

		char_instr.set_metadata(range_metadata_node, range_metadata_id)
	}

	fn add_puts_io_attributes(
		&self,
		call: CallSiteValue<'ctx>,
		array_ty: ArrayType<'ctx>,
		array_len: u64,
	) {
		call.set_tail_call(true);

		let byref_attr = self
			.context()
			.create_type_attribute(Attribute::get_named_enum_kind_id("byref"), array_ty.into());

		let deref_attr = self.context().create_enum_attribute(
			Attribute::get_named_enum_kind_id("dereferenceable"),
			array_len + 1,
		);

		let align_attr = self
			.context()
			.create_enum_attribute(Attribute::get_named_enum_kind_id("align"), 1);

		for attribute in [byref_attr, deref_attr, align_attr] {
			call.add_attribute(AttributeLoc::Param(0), attribute);
		}
	}

	pub fn input_into_cell(&self) -> Result<(), LlvmAssemblyError> {
		let context = self.context();

		let i8_type = context.i8_type();

		let getchar_call =
			self.builder
				.build_call(self.functions.getchar, &[], "input_into_cell_call")?;

		getchar_call.set_tail_call(true);

		let getchar_value = getchar_call
			.try_as_basic_value()
			.unwrap_left()
			.into_int_value();

		self.add_range_io_metadata(context, getchar_value, u8::MIN.into(), u8::MAX.into())?;

		let truncated_value =
			self.builder
				.build_int_truncate(getchar_value, i8_type, "input_into_cell_truncate")?;

		self.store(truncated_value, 0, "input_into_cell")
	}

	fn call_puts(
		&self,
		array_ptr: PointerValue<'ctx>,
		array_len: IntValue<'ctx>,
		fn_name: impl Display,
	) -> Result<CallSiteValue<'ctx>, LlvmAssemblyError> {
		Ok(self.builder.build_call(
			self.functions.puts,
			&[array_ptr.into(), array_len.into()],
			&format!("{fn_name}_puts_call"),
		)?)
	}
}
