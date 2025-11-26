mod instr;
mod metadata;
mod utils;

use std::{cell::RefCell, fs, path::Path};

use frick_instructions::{BrainInstruction, BrainInstructionType};
use frick_spec::TAPE_SIZE;
use frick_utils::Convert as _;
use inkwell::{
	basic_block::BasicBlock,
	builder::Builder,
	context::{AsContextRef, Context},
	debug_info::AsDIScope,
	llvm_sys::prelude::LLVMContextRef,
	module::{FlagBehavior, Module},
	targets::{TargetMachine, TargetTriple},
	values::{BasicMetadataValueEnum, BasicValueEnum},
};

pub use self::utils::AssemblerFunctions;
use self::utils::{AssemblerDebugBuilder, AssemblerPointers};
use super::AssemblyError;
use crate::{ContextGetter as _, ModuleExt as _};

pub struct InnerAssembler<'ctx> {
	file_data: String,
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: AssemblerFunctions<'ctx>,
	pointers: AssemblerPointers<'ctx>,
	target_machine: TargetMachine,
	debug_builder: AssemblerDebugBuilder<'ctx>,
	registers: RefCell<Vec<BasicValueEnum<'ctx>>>,
	loop_blocks: RefCell<Vec<LoopBlocks<'ctx>>>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(
		context: &'ctx Context,
		target_machine: TargetMachine,
		target_triple: TargetTriple,
		cpu_name: &str,
		cpu_features: &str,
		file_path: &Path,
	) -> Result<Self, AssemblyError> {
		let module = context.create_module("frick\0");
		let functions = AssemblerFunctions::new(context, &module, cpu_name, cpu_features)?;
		let builder = context.create_builder();

		let target_data = target_machine.get_target_data();
		let data_layout = target_data.get_data_layout();

		module.set_new_debug_format(true);
		module.set_data_layout(&data_layout);
		module.set_triple(&target_triple);

		let basic_block = context.append_basic_block(functions.main, "entry\0");
		builder.position_at_end(basic_block);

		let pointers = AssemblerPointers::new(&module, &builder)?;

		pointers.setup(&builder, &functions)?;

		let start_block = context.append_basic_block(functions.main, "start\0");
		builder.build_unconditional_branch(start_block)?;
		builder.position_at_end(start_block);

		let debug_metadata_version = {
			let i32_type = context.i32_type();

			i32_type.const_int(
				inkwell::debug_info::debug_metadata_version().convert::<u64>(),
				false,
			)
		};

		module.add_basic_value_flag(
			"Debug Info Version",
			FlagBehavior::Warning,
			debug_metadata_version,
		);

		let (file_name, directory) = {
			assert!(file_path.is_file());

			let file_name = file_path
				.file_name()
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			let directory = file_path
				.parent()
				.and_then(|s| s.canonicalize().ok())
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			(file_name, directory)
		};

		let debug_builder = AssemblerDebugBuilder::new(&module, &file_name, &directory)?;

		debug_builder.declare_subprograms(&functions)?;

		let debug_loc = debug_builder.create_debug_location(
			module.get_context(),
			1,
			0,
			functions
				.main
				.get_subprogram()
				.unwrap()
				.as_debug_info_scope(),
			None,
		);

		builder.set_current_debug_location(debug_loc);
		module.set_source_file_name(&file_name);

		let file_data = fs::read_to_string(file_path)?;

		Ok(Self {
			file_data,
			module,
			builder,
			functions,
			pointers,
			target_machine,
			debug_builder,
			registers: RefCell::default(),
			loop_blocks: RefCell::default(),
		})
	}

	pub fn assemble(
		self,
		instrs: &[BrainInstruction],
	) -> Result<(Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine), AssemblyError> {
		tracing::debug!("writing instructions");

		let instrs_span = tracing::info_span!("instrs").entered();
		self.instrs(instrs)?;
		drop(instrs_span);

		tracing::debug!("declaring variables");
		self.debug_builder
			.declare_variables(self.context(), self.pointers)?;

		self.builder.unset_current_debug_location();

		let context = self.context();

		let i64_type = context.i64_type();

		let i64_size = i64_type.const_int(8, false);

		let tape_size = i64_type.const_int(TAPE_SIZE as u64, false);

		tracing::debug!("ending lifetimes in exit block");
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				tape_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers.tape.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;
		self.builder.build_call(
			self.functions.lifetime.end,
			&[
				i64_size.convert::<BasicMetadataValueEnum<'ctx>>(),
				self.pointers
					.pointer
					.convert::<BasicMetadataValueEnum<'ctx>>(),
			],
			"\0",
		)?;

		self.builder.build_return(None)?;

		self.debug_builder.di_builder.finalize();

		Ok(self.into_parts())
	}

	#[allow(clippy::never_loop)]
	fn instrs(&self, instrs: &[BrainInstruction]) -> Result<(), AssemblyError> {
		let line_positions = line_numbers::LinePositions::from(self.file_data.as_str());

		for i in instrs {
			let i_range = i.span();

			let line_span = line_positions.from_region(i_range.start, i_range.end)[0];
			let debug_loc = self.debug_builder.create_debug_location(
				self.context(),
				(line_span.line.as_usize() + 1) as u32,
				line_span.start_col + 1,
				self.debug_builder.get_current_scope(),
				None,
			);

			self.builder.set_current_debug_location(debug_loc);

			if !self.compile_instruction(i.instr())? {
				return Err(AssemblyError::NotImplemented(i.instr()));
			}
		}

		Ok(())
	}

	fn compile_instruction(&self, instr: BrainInstructionType) -> Result<bool, AssemblyError> {
		match instr {
			BrainInstructionType::LoadCellIntoRegister {
				pointer_reg,
				output_reg,
			} => self.load_cell_into_register(pointer_reg, output_reg)?,
			BrainInstructionType::StoreRegisterIntoCell {
				value_reg,
				pointer_reg,
			} => self.store_register_into_cell(value_reg, pointer_reg)?,
			BrainInstructionType::StoreImmediateIntoRegister { output_reg, imm } => {
				self.store_immediate_into_register(output_reg, imm)?;
			}
			BrainInstructionType::LoadTapePointerIntoRegister { output_reg } => {
				self.load_tape_pointer_into_register(output_reg)?;
			}
			BrainInstructionType::StoreRegisterIntoTapePointer { input_reg } => {
				self.store_register_into_tape_pointer(input_reg)?;
			}
			BrainInstructionType::CalculateTapeOffset {
				tape_pointer_reg,
				output_reg,
			} => self.calculate_tape_offset(tape_pointer_reg, output_reg)?,
			BrainInstructionType::PerformBinaryRegisterOperation {
				lhs_reg,
				rhs_reg,
				output_reg,
				op,
			} => self.perform_binary_register_operation(lhs_reg, rhs_reg, output_reg, op)?,
			BrainInstructionType::InputIntoRegister { output_reg } => {
				self.input_into_register(output_reg)?;
			}
			BrainInstructionType::OutputFromRegister { input_reg } => {
				self.output_from_register(input_reg)?;
			}
			BrainInstructionType::StartLoop => self.start_loop()?,
			BrainInstructionType::EndLoop => self.end_loop()?,
			BrainInstructionType::CompareRegisterToRegister {
				lhs_reg,
				rhs_reg,
				output_reg,
			} => self.compare_register_to_register(lhs_reg, rhs_reg, output_reg)?,
			BrainInstructionType::JumpIf { input_reg } => self.jump_if(input_reg)?,
			BrainInstructionType::JumpToHeader => self.jump_to_header()?,
			_ => return Ok(false),
		}

		Ok(true)
	}

	fn into_parts(self) -> (Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine) {
		(self.module, self.functions, self.target_machine)
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for InnerAssembler<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.module.get_context().as_ctx_ref()
	}
}

#[derive(Debug, Clone, Copy)]
struct LoopBlocks<'ctx> {
	header: BasicBlock<'ctx>,
	body: BasicBlock<'ctx>,
	exit: BasicBlock<'ctx>,
}
