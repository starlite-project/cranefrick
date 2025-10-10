use std::{
	mem,
	ops::{Deref, DerefMut},
};

use frick_spec::TAPE_SIZE;
use inkwell::{
	AddressSpace,
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DWARFEmissionKind,
		DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::Module,
	values::{InstructionValue, PointerValue},
};

use super::{AssemblerFunctions, AssemblerPointers, OUTPUT_ARRAY_LEN};
use crate::{AssemblyError, ContextGetter as _};

pub struct AssemblerDebugBuilder<'ctx> {
	pub di_builder: DebugInfoBuilder<'ctx>,
	pub compile_unit: DICompileUnit<'ctx>,
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

		Ok(Self {
			di_builder,
			compile_unit,
		})
	}

	pub fn declare_variables(
		&self,
		functions: AssemblerFunctions<'ctx>,
		pointers: AssemblerPointers<'ctx>,
	) -> Result<(), AssemblyError> {
		let main_subprogram = functions.main.get_subprogram().unwrap();

		let debug_loc = self.create_debug_location(
			functions.context(),
			1,
			0,
			main_subprogram.as_debug_info_scope(),
			None,
		);

		let u8_type = self
			.create_basic_type("u8", mem::size_of::<u8>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let u8_array_type = self
			.create_array_type(
				u8_type,
				mem::size_of::<[u8; TAPE_SIZE]>() as u64 * 8,
				mem::align_of::<[u8; TAPE_SIZE]>() as u32 * 8,
				&[],
			)
			.as_type();

		let tape_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"tape",
			self.compile_unit.get_file(),
			0,
			u8_array_type,
			false,
			i32::ZERO,
			mem::align_of::<[u8; TAPE_SIZE]>() as u32 * 8,
		);

		let right_after_tape_alloca = get_instruction_after_alloca(pointers.tape)?;

		self.insert_declare_before_instruction(
			pointers.tape,
			Some(tape_variable),
			None,
			debug_loc,
			right_after_tape_alloca,
		);

		let pointer_type = self
			.create_basic_type("usize", mem::size_of::<usize>() as u64 * 8, 7, i32::ZERO)?
			.as_type();

		let pointer_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"pointer",
			self.compile_unit.get_file(),
			0,
			pointer_type,
			false,
			i32::ZERO,
			mem::align_of::<usize>() as u32 * 8,
		);

		let right_after_pointer_alloca = get_instruction_after_alloca(pointers.pointer)?;

		self.insert_declare_before_instruction(
			pointers.pointer,
			Some(pointer_variable),
			None,
			debug_loc,
			right_after_pointer_alloca,
		);

		let output_array_type = self
			.create_array_type(
				u8_type,
				mem::size_of::<[u8; OUTPUT_ARRAY_LEN as usize]>() as u64 * 8,
				mem::align_of::<[u8; OUTPUT_ARRAY_LEN as usize]>() as u32 * 8,
				&[],
			)
			.as_type();

		let output_variable = self.create_auto_variable(
			main_subprogram.as_debug_info_scope(),
			"output",
			self.compile_unit.get_file(),
			0,
			output_array_type,
			false,
			i32::ZERO,
			mem::align_of::<[u8; OUTPUT_ARRAY_LEN as usize]>() as u32 * 8,
		);

		let right_after_output_alloca = get_instruction_after_alloca(pointers.output)?;

		self.insert_declare_before_instruction(
			pointers.output,
			Some(output_variable),
			None,
			debug_loc,
			right_after_output_alloca,
		);

		Ok(())
	}

	pub fn declare_subprograms(
		&self,
		functions: AssemblerFunctions<'ctx>,
	) -> Result<(), AssemblyError> {
		let main_subroutine_type =
			self.create_subroutine_type(self.compile_unit.get_file(), None, &[], i32::ZERO);

		let main_subprogram = self.create_function(
			self.compile_unit.as_debug_info_scope(),
			"main",
			None,
			self.compile_unit.get_file(),
			0,
			main_subroutine_type,
			true,
			true,
			0,
			i32::PUBLIC,
			true,
		);

		functions.main.set_subprogram(main_subprogram);

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
			.create_basic_type("char", mem::size_of::<u32>() as u64 * 8, 7, i32::ZERO)?
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
			"puts",
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
