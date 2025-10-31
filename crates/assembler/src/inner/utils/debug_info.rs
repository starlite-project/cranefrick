use std::{
	mem,
	ops::{Deref, DerefMut},
};

use frick_spec::TAPE_SIZE;
use inkwell::{
	context::ContextRef,
	debug_info::{
		AsDIScope as _, DICompileUnit, DIFlagsConstants as _, DILocalVariable, DISubprogram,
		DWARFEmissionKind, DWARFSourceLanguage, DebugInfoBuilder,
	},
	module::Module,
	values::{InstructionValue, PointerValue},
};

use super::{AssemblerFunctions, AssemblerPointers};
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

		let char_type = self
			.create_basic_type("char", mem::size_of::<u32>() as u64 * 8, 8, i32::ZERO)?
			.as_type();

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
		pointers: AssemblerPointers<'ctx>,
	) -> Result<(), AssemblyError> {
		let entry_block = pointers
			.tape
			.as_instruction()
			.and_then(InstructionValue::get_parent)
			.unwrap();

		let debug_loc = self.create_debug_location(
			context,
			1,
			0,
			self.main_subprogram.as_debug_info_scope(),
			None,
		);

		self.insert_declare_at_end(
			pointers.tape,
			Some(self.variables.tape),
			None,
			debug_loc,
			entry_block,
		);

		self.insert_declare_at_end(
			pointers.pointer,
			Some(self.variables.pointer),
			None,
			debug_loc,
			entry_block,
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
			tape_align_in_bits,
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

		Ok(Self { tape, pointer })
	}
}

// Leaving this here in case I want to use it again
#[allow(unused)]
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
