use frick_spec::TAPE_SIZE;
use inkwell::{
	IntPredicate,
	types::VectorType,
	values::{IntValue, VectorValue},
};

use crate::{
	AssemblyError, ContextGetter as _,
	inner::{InnerAssembler, utils::CalculatedOffset},
};

impl<'ctx> InnerAssembler<'ctx> {
	#[tracing::instrument(skip(self))]
	pub fn move_pointer(&self, offset: i32) -> Result<(), AssemblyError> {
		let wrapped_ptr = self.offset_pointer(offset)?;

		let store_instr = self.store_into(wrapped_ptr, self.pointers.pointer)?;

		let current_debug_loc = self.builder.get_current_debug_location().unwrap();

		self.pointer_setting_instructions.borrow_mut().push((
			store_instr,
			wrapped_ptr,
			current_debug_loc,
		));

		Ok(())
	}

	#[tracing::instrument(skip(self))]
	pub fn resolve_offset(
		&self,
		offset: CalculatedOffset<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		match offset {
			CalculatedOffset::Calculated(offset) => Ok(offset),
			CalculatedOffset::Raw(offset) => self.offset_pointer(offset),
		}
	}

	#[tracing::instrument(skip(self))]
	pub fn offset_many_pointers(
		&self,
		offsets: &[i32],
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let context = self.context();

		let i32_type = context.i32_type();
		let i64_type = context.i64_type();
		let ptr_int_type = self.pointers.pointer_ty;
		let i32_vec_type = i32_type.vec_type(offsets.len() as u32);
		let ptr_int_vec_type = ptr_int_type.vec_type(offsets.len() as u32);

		let vec_of_pointers = {
			let i64_zero = i64_type.const_zero();

			let pointer_value = self.load_from(ptr_int_type, self.pointers.pointer)?;

			let tmp = self.builder.build_insert_element(
				ptr_int_vec_type.get_poison(),
				pointer_value,
				i64_zero,
				"offset_many_pointers_insert_element",
			)?;

			self.builder.build_shuffle_vector(
				tmp,
				ptr_int_vec_type.get_poison(),
				i32_vec_type.const_zero(),
				"offset_many_pointers_shuffle_vector",
			)?
		};

		if offsets.iter().all(|x| matches!(x, 0)) {
			Ok(vec_of_pointers)
		} else {
			let vec_of_offset_values = {
				let vec_of_offsets = offsets
					.iter()
					.map(|&i| ptr_int_type.const_int(i as u64, false))
					.collect::<Vec<_>>();

				VectorType::const_vector(&vec_of_offsets)
			};

			let vec_of_offset_pointers = self.builder.build_int_nsw_add(
				vec_of_pointers,
				vec_of_offset_values,
				"offset_many_pointers_add\0",
			)?;

			if offsets.iter().all(|x| x.is_positive()) {
				self.wrap_many_pointers_positive(vec_of_offset_pointers)
			} else if offsets.iter().all(|x| x.is_negative()) {
				self.wrap_many_pointers_negative(vec_of_offset_pointers)
			} else {
				self.wrap_many_pointers_mixed(vec_of_offset_pointers)
			}
		}
	}

	#[tracing::instrument(skip(self))]
	pub fn offset_pointer(&self, offset: i32) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.pointers.pointer_ty;
		let offset_value = ptr_int_type.const_int(offset as u64, false);

		let current_ptr = self.load_from(ptr_int_type, self.pointers.pointer)?;

		if matches!(offset, 0) {
			Ok(current_ptr)
		} else {
			let offset_ptr = self.builder.build_int_nsw_add(
				current_ptr,
				offset_value,
				"offset_pointer_add\0",
			)?;

			self.wrap_pointer(offset_ptr, offset > 0)
		}
	}

	#[tracing::instrument(skip(self))]
	pub fn wrap_pointer(
		&self,
		offset_ptr: IntValue<'ctx>,
		is_positive: bool,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		if is_positive {
			self.wrap_pointer_positive(offset_ptr)
		} else {
			self.wrap_pointer_negative(offset_ptr)
		}
	}

	#[tracing::instrument(skip(self))]
	fn wrap_pointer_positive(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.pointers.pointer_ty;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		Ok(self.builder.build_int_unsigned_rem(
			offset_ptr,
			tape_size,
			"wrap_pointer_positive_urem\0",
		)?)
	}

	#[tracing::instrument(skip(self))]
	fn wrap_pointer_negative(
		&self,
		offset_ptr: IntValue<'ctx>,
	) -> Result<IntValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.pointers.pointer_ty;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let tmp = self.builder.build_int_signed_rem(
			offset_ptr,
			tape_size,
			"wrap_pointer_negative_srem\0",
		)?;

		let added_offset =
			self.builder
				.build_int_nsw_add(tmp, tape_size, "wrap_pointer_negative_add\0")?;

		let cmp = self.builder.build_int_compare(
			IntPredicate::SLT,
			tmp,
			ptr_int_type.const_zero(),
			"wrap_pointer_negative_cmp\0",
		)?;

		Ok(self
			.builder
			.build_select(cmp, added_offset, tmp, "wrap_pointer_negative_select\0")?
			.into_int_value())
	}

	#[tracing::instrument(skip(self))]
	fn wrap_many_pointers_mixed(
		&self,
		vec_of_offset_pointers: VectorValue<'ctx>,
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let ptr_int_type = self.pointers.pointer_ty;
		let ptr_int_vec_type = ptr_int_type.vec_type(vec_of_offset_pointers.get_type().get_size());

		let vec_of_tape_sizes = {
			let vec_of_values = vec![
				ptr_int_type.const_int(TAPE_SIZE as u64, false);
				vec_of_offset_pointers.get_type().get_size() as usize
			];

			VectorType::const_vector(&vec_of_values)
		};

		let signed_rem = self.builder.build_int_signed_rem(
			vec_of_offset_pointers,
			vec_of_tape_sizes,
			"wrap_many_pointers_mixed_srem\0",
		)?;

		let unsigned_rem = self.builder.build_int_unsigned_rem(
			vec_of_offset_pointers,
			vec_of_tape_sizes,
			"wrap_many_pointers_mixed_urem\0",
		)?;

		let added_offset = self.builder.build_int_nsw_add(
			signed_rem,
			vec_of_tape_sizes,
			"wrap_many_pointers_mixed_add\0",
		)?;

		let is_negative_vec = self.builder.build_int_compare(
			IntPredicate::SLT,
			vec_of_offset_pointers,
			ptr_int_vec_type.const_zero(),
			"wrap_many_pointers_mixed_cmp\0",
		)?;

		Ok(self
			.builder
			.build_select(
				is_negative_vec,
				added_offset,
				unsigned_rem,
				"wrap_many_pointers_mixed_select\0",
			)?
			.into_vector_value())
	}

	#[tracing::instrument(skip(self))]
	fn wrap_many_pointers_positive(
		&self,
		vec_of_offset_pointers: VectorValue<'ctx>,
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let vec_of_tape_sizes =
			self.get_vec_of_tape_sizes(vec_of_offset_pointers.get_type().get_size());

		Ok(self.builder.build_int_unsigned_rem(
			vec_of_offset_pointers,
			vec_of_tape_sizes,
			"wrap_many_pointers_positive_urem\0",
		)?)
	}

	#[tracing::instrument(skip(self))]
	fn wrap_many_pointers_negative(
		&self,
		vec_of_offset_pointers: VectorValue<'ctx>,
	) -> Result<VectorValue<'ctx>, AssemblyError> {
		let vec_size = vec_of_offset_pointers.get_type().get_size();

		let ptr_int_type = self.pointers.pointer_ty;
		let ptr_int_vec_type = ptr_int_type.vec_type(vec_size);

		let vec_of_tape_sizes = self.get_vec_of_tape_sizes(vec_size);

		let tmp = self.builder.build_int_signed_rem(
			vec_of_offset_pointers,
			vec_of_tape_sizes,
			"wrap_many_pointers_negative_srem\0",
		)?;

		let added_offset = self.builder.build_int_nsw_add(
			tmp,
			vec_of_tape_sizes,
			"wrap_many_pointers_negative_add\0",
		)?;

		let cmp = self.builder.build_int_compare(
			IntPredicate::SLT,
			tmp,
			ptr_int_vec_type.const_zero(),
			"wrap_many_pointers_negative_cmp\0",
		)?;

		Ok(self
			.builder
			.build_select(
				cmp,
				added_offset,
				tmp,
				"wrap_many_pointers_negative_select\0",
			)?
			.into_vector_value())
	}

	fn get_vec_of_tape_sizes(&self, vec_size: u32) -> VectorValue<'ctx> {
		let ptr_int_type = self.pointers.pointer_ty;

		let tape_size = ptr_int_type.const_int(TAPE_SIZE as u64, false);

		let vec_of_values = vec![tape_size; vec_size as usize];

		VectorType::const_vector(&vec_of_values)
	}
}
