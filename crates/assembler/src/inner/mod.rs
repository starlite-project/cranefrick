mod instr;
mod utils;

use std::{cell::RefCell, fs, path::Path};

use frick_instructions::{BrainInstructionType, Reg, ToInstructions as _};
use frick_operations::BrainOperation;
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
	values::{BasicMetadataValueEnum, BasicValueEnum, IntValue},
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
	pointer_register: RefCell<Option<IntValue<'ctx>>>,
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

		let pointers = AssemblerPointers::new(&module, &builder, &target_data)?;

		pointers.setup(&builder, &functions)?;

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
			pointer_register: RefCell::default(),
			loop_blocks: RefCell::default(),
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainOperation],
	) -> Result<(Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine), AssemblyError> {
		tracing::debug!("writing instructions");
		let ops_span = tracing::info_span!("ops").entered();
		self.ops(ops)?;
		drop(ops_span);

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
	fn ops(&self, ops: &[BrainOperation]) -> Result<(), AssemblyError> {
		let line_positions = line_numbers::LinePositions::from(self.file_data.as_str());

		for op in ops {
			let instructions = op.to_instructions();

			for i in instructions {
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
					return Err(AssemblyError::NotImplemented(op.op().clone(), i.instr()));
				}
			}
		}

		Ok(())
	}

	fn compile_instruction(&self, instr: BrainInstructionType) -> Result<bool, AssemblyError> {
		match instr {
			BrainInstructionType::LoadCellIntoRegister(Reg(reg)) => {
				self.load_cell_into_register(reg)?;
			}
			BrainInstructionType::StoreRegisterIntoCell(Reg(reg)) => {
				self.store_register_into_cell(reg)?;
			}
			BrainInstructionType::StoreImmediateIntoCell(imm) => {
				self.store_immediate_into_cell(imm)?;
			}
			BrainInstructionType::ChangeRegisterByImmediate(Reg(reg), imm) => {
				self.change_register_by_immediate(reg, imm)?;
			}
			BrainInstructionType::InputIntoRegister(Reg(reg)) => self.input_into_register(reg)?,
			BrainInstructionType::OutputFromRegister(Reg(reg)) => self.output_from_register(reg)?,
			BrainInstructionType::LoadPointer => self.load_pointer()?,
			BrainInstructionType::OffsetPointer(offset) => self.offset_pointer(offset)?,
			BrainInstructionType::StorePointer => self.store_pointer()?,
			BrainInstructionType::StartLoop => self.start_loop()?,
			BrainInstructionType::EndLoop => self.end_loop()?,
			BrainInstructionType::CompareRegisterToImmediate {
				input_reg: Reg(input_reg),
				output_reg: Reg(output_reg),
				imm,
			} => self.compare_reg_to_immediate(input_reg, output_reg, imm)?,
			BrainInstructionType::JumpIf {
				input_reg: Reg(input_reg),
			} => self.jump_if(input_reg)?,
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
