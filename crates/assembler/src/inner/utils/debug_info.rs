use std::{
	mem,
	ops::{Deref, DerefMut},
};

use frick_spec::TAPE_SIZE;
use inkwell::{
	AddressSpace,
	context::ContextRef,
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DILocalVariable, DISubprogram,
		DWARFEmissionKind, DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::Module,
	types::IntType,
	values::{InstructionValue, PointerValue},
};

use super::{AssemblerFunctions, AssemblerPointers, OUTPUT_ARRAY_LEN};
use crate::AssemblyError;

pub struct AssemblerDebugBuilder<'ctx> {
	pub di_builder: DebugInfoBuilder<'ctx>,
	pub compile_unit: DICompileUnit<'ctx>,
	pub variables: AssemblerDebugVariables<'ctx>,
	pub main_subprogram: DISubprogram<'ctx>,
}

impl<'ctx> AssemblerDebugBuilder<'ctx> {
	pub fn new(
		module: &Module<'ctx>,
		file_name: &str,
		directory: &str,
	) -> Result<Self, AssemblyError> {
		let (di_builder, compile_unit) = module.create_debug_info_builder(
			true,
			DWARFSourceLanguage::C,
			file_name,
			directory,
			"frick",
			false,
			"",
			0,
			"",
			DWARFEmissionKind::Full,
			0,
			false,
			false,
			"",
			"",
		);

		let main_subroutine_type =
			di_builder.create_subroutine_type(compile_unit.get_file(), None, &[], i32::ZERO);

		let main_subprogram = di_builder.create_function(
			compile_unit.as_debug_info_scope(),
			"main",
			None,
			compile_unit.get_file(),
			0,
			main_subroutine_type,
			true,
			true,
			0,
			i32::PUBLIC,
			true,
		);

		let variables = AssemblerDebugVariables::new(&di_builder, compile_unit, main_subprogram)?;

		Ok(Self {
			di_builder,
			compile_unit,
			variables,
			main_subprogram,
		})
	}

	pub fn declare_subprograms(
		&self,
		functions: &AssemblerFunctions<'ctx>,
	) -> Result<(), AssemblyError> {
		functions.main.set_subprogram(self.main_subprogram);

		let u8_type = self
			.create_basic_type("u8", mem::size_of::<u8>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let u8_ptr_type = self
			.create_pointer_type(
				"ptr(u8)",
				u8_type,
				mem::size_of::<*const u8>() as u64 * 8,
				mem::align_of::<*const u8>() as u32 * 8,
				AddressSpace::default(),
			)
			.as_type();

		let char_type = self
			.create_basic_type("char", mem::size_of::<u32>() as u64 * 8, 8, i32::ZERO)?
			.as_type();

		let usize_type = self
			.create_basic_type("usize", mem::size_of::<usize>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let puts_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(char_type),
			&[u8_ptr_type, usize_type],
			i32::ZERO,
		);

		let puts_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"frick_puts",
			None,
			self.compile_unit.get_file(),
			0,
			puts_subroutine_type,
			true,
			true,
			0,
			i32::ZERO,
			true,
		);

		functions.puts.set_subprogram(puts_subprogram);

		let putchar_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(char_type),
			&[char_type],
			i32::ZERO,
		);

		let putchar_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"putchar",
			Some("rust_putchar"),
			self.compile_unit.get_file(),
			0,
			putchar_subroutine_type,
			false,
			false,
			0,
			i32::ZERO,
			true,
		);

		functions.putchar.set_subprogram(putchar_subprogram);

		let getchar_subroutine_type = self.create_subroutine_type(
			self.compile_unit.get_file(),
			Some(char_type),
			&[],
			i32::ZERO,
		);

		let getchar_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"getchar",
			Some("rust_getchar"),
			self.compile_unit.get_file(),
			0,
			getchar_subroutine_type,
			false,
			false,
			0,
			i32::ZERO,
			true,
		);

		functions.getchar.set_subprogram(getchar_subprogram);

		Ok(())
	}

	pub fn declare_variables(
		&self,
		context: ContextRef<'ctx>,
		ptr_int_type: IntType<'ctx>,
		pointers: AssemblerPointers<'ctx>,
	) -> Result<(), AssemblyError> {
		let i8_type = context.i8_type();

		let debug_loc = self.create_debug_location(
			context,
			1,
			0,
			self.main_subprogram.as_debug_info_scope(),
			None,
		);

		let tape_value = {
			let i8_array_type = i8_type.array_type(TAPE_SIZE as u32);

			i8_array_type.const_zero()
		};

		let right_after_tape_alloca = get_instruction_after_alloca(pointers.tape)?;

		self.insert_declare_before_instruction(
			pointers.tape,
			Some(self.variables.tape),
			None,
			debug_loc,
			right_after_tape_alloca,
		);

		self.insert_dbg_value_before(
			tape_value.into(),
			self.variables.tape,
			None,
			debug_loc,
			right_after_tape_alloca,
		);

		let pointer_value = ptr_int_type.const_zero();

		let right_after_pointer_alloca = get_instruction_after_alloca(pointers.pointer)?;

		self.insert_declare_before_instruction(
			pointers.pointer,
			Some(self.variables.pointer),
			None,
			debug_loc,
			right_after_pointer_alloca,
		);

		self.insert_dbg_value_before(
			pointer_value.into(),
			self.variables.pointer,
			None,
			debug_loc,
			right_after_pointer_alloca,
		);

		let output_array_value = {
			let output_array_type = i8_type.array_type(OUTPUT_ARRAY_LEN);

			output_array_type.get_undef()
		};

		let right_after_output_alloca = get_instruction_after_alloca(pointers.output)?;

		self.insert_declare_before_instruction(
			pointers.output,
			Some(self.variables.output),
			None,
			debug_loc,
			right_after_output_alloca,
		);

		self.insert_dbg_value_before(
			output_array_value.into(),
			self.variables.output,
			None,
			debug_loc,
			right_after_output_alloca,
		);

		Ok(())
	}
}

impl<'ctx> Deref for AssemblerDebugBuilder<'ctx> {
	type Target = DebugInfoBuilder<'ctx>;

	fn deref(&self) -> &Self::Target {
		&self.di_builder
	}
}

impl DerefMut for AssemblerDebugBuilder<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.di_builder
	}
}

// These should match the AssemblerPointers struct
pub struct AssemblerDebugVariables<'ctx> {
	pub tape: DILocalVariable<'ctx>,
	pub pointer: DILocalVariable<'ctx>,
	pub output: DILocalVariable<'ctx>,
}

impl<'ctx> AssemblerDebugVariables<'ctx> {
	#[allow(clippy::single_range_in_vec_init)]
	fn new(
		debug_builder: &DebugInfoBuilder<'ctx>,
		compile_unit: DICompileUnit<'ctx>,
		main_subprogram: DISubprogram<'ctx>,
	) -> Result<Self, AssemblyError> {
		let u8_type = debug_builder
			.create_basic_type("u8", mem::size_of::<u8>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let tape_align_in_bits = mem::align_of::<[u8; TAPE_SIZE]>() as u32 * 8;

		let tape_array_type = debug_builder
			.create_array_type(
				u8_type,
				mem::size_of::<[u8; TAPE_SIZE]>() as u64 * 8,
				tape_align_in_bits,
				&[0..(TAPE_SIZE as i64)],
			)
			.as_type();

		let tape = debug_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"tape",
			compile_unit.get_file(),
			0,
			tape_array_type,
			false,
			i32::ZERO,
			tape_align_in_bits * 16,
		);

		let pointer_type = debug_builder
			.create_basic_type("usize", mem::size_of::<usize>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let pointer = debug_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"pointer",
			compile_unit.get_file(),
			0,
			pointer_type,
			false,
			i32::ZERO,
			mem::align_of::<usize>() as u32 * 8,
		);

		let output_align_in_bits = mem::align_of::<[u8; OUTPUT_ARRAY_LEN as usize]>() as u32 * 8;

		let output_array_type = debug_builder
			.create_array_type(
				u8_type,
				mem::size_of::<[u8; OUTPUT_ARRAY_LEN as usize]>() as u64 * 8,
				output_align_in_bits,
				&[0..i64::from(OUTPUT_ARRAY_LEN)],
			)
			.as_type();

		let output = debug_builder.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"output",
			compile_unit.get_file(),
			0,
			output_array_type,
			false,
			i32::ZERO,
			output_align_in_bits * 16,
		);

		Ok(Self {
			tape,
			pointer,
			output,
		})
	}
}

fn get_instruction_after_alloca(
	alloca: PointerValue<'_>,
) -> Result<InstructionValue<'_>, AssemblyError> {
	let alloca_name = || alloca.get_name().to_string_lossy().into_owned();
	let alloca_instr =
		alloca
			.as_instruction()
			.ok_or_else(|| AssemblyError::MissingPointerInstruction {
				alloca_name: alloca_name(),
				looking_after: false,
			})?;

	alloca_instr
		.get_next_instruction()
		.ok_or_else(|| AssemblyError::MissingPointerInstruction {
			alloca_name: alloca_name(),
			looking_after: true,
		})
}
