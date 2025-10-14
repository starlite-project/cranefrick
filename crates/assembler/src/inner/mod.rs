mod impls;
mod utils;

use std::path::Path;

use frick_ir::{BrainIr, SubType};
use frick_spec::TAPE_SIZE;
use frick_utils::GetOrZero as _;
use inkwell::{
	basic_block::BasicBlock,
	builder::Builder,
	context::{AsContextRef, Context},
	debug_info::AsDIScope,
	llvm_sys::prelude::LLVMContextRef,
	module::{FlagBehavior, Linkage, Module},
	targets::TargetMachine,
	types::IntType,
	values::{BasicValue, GlobalValue},
};

pub use self::utils::AssemblerFunctions;
use self::utils::{AssemblerDebugBuilder, AssemblerPointers};
use super::AssemblyError;
use crate::{ContextExt as _, ContextGetter as _};

pub struct InnerAssembler<'ctx> {
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	functions: AssemblerFunctions<'ctx>,
	pointers: AssemblerPointers<'ctx>,
	ptr_int_type: IntType<'ctx>,
	target_machine: TargetMachine,
	debug_builder: AssemblerDebugBuilder<'ctx>,
	catch_block: BasicBlock<'ctx>,
}

impl<'ctx> InnerAssembler<'ctx> {
	pub fn new(
		context: &'ctx Context,
		target_machine: TargetMachine,
		path: Option<&Path>,
	) -> Result<Self, AssemblyError> {
		let module = context.create_module("frick\0");
		let functions = AssemblerFunctions::new(context, &module)?;
		let builder = context.create_builder();

		let target_data = target_machine.get_target_data();

		let data_layout = target_data.get_data_layout();

		let target_triple = {
			let default_target = TargetMachine::get_default_triple();

			TargetMachine::normalize_triple(&default_target)
		};

		module.set_data_layout(&data_layout);
		module.set_triple(&target_triple);

		let basic_block = context.append_basic_block(functions.main, "entry\0");
		builder.position_at_end(basic_block);

		let catch_block = context.append_basic_block(functions.main, "lpad\0");

		let (pointers, ptr_int_type) =
			AssemblerPointers::new(&module, functions, &builder, &target_data)?;

		let debug_metadata_version = {
			let i32_type = context.i32_type();

			i32_type.const_int(inkwell::debug_info::debug_metadata_version().into(), false)
		};

		module.add_basic_value_flag(
			"Debug Info Version",
			FlagBehavior::Warning,
			debug_metadata_version,
		);

		let (file_name, directory) = if let Some(path) = path {
			assert!(path.is_file());

			let file_name = path
				.file_name()
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			let directory = path
				.parent()
				.and_then(|s| s.canonicalize().ok())
				.map(|s| s.to_string_lossy().into_owned())
				.unwrap_or_default();

			(file_name, directory)
		} else {
			("frick_source_file.bf".to_owned(), "/".to_owned())
		};

		let debug_builder = AssemblerDebugBuilder::new(&module, &file_name, &directory)?;

		debug_builder.declare_subprograms(functions)?;

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

		Ok(Self {
			module,
			builder,
			functions,
			pointers,
			ptr_int_type,
			target_machine,
			debug_builder,
			catch_block,
		})
	}

	pub fn assemble(
		self,
		ops: &[BrainIr],
	) -> Result<(Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine), AssemblyError> {
		assert!(TAPE_SIZE.is_power_of_two());

		self.ops(ops, 1)?;

		self.debug_builder
			.declare_variables(self.context(), self.pointers)?;

		let context = self.context();

		let i64_type = context.i64_type();
		let i32_type = context.i32_type();
		let ptr_type = context.default_ptr_type();

		let i64_size = i64_type.const_int(8, false);

		let tape_size = i64_type.const_int(TAPE_SIZE as u64, false);

		self.builder.build_call(
			self.functions.lifetime.end,
			&[tape_size.into(), self.pointers.tape.into()],
			"\0",
		)?;
		self.builder.build_call(
			self.functions.lifetime.end,
			&[i64_size.into(), self.pointers.pointer.into()],
			"\0",
		)?;

		self.builder.build_return(None)?;

		self.builder.unset_current_debug_location();

		let last_basic_block = self.functions.main.get_last_basic_block().unwrap();

		if last_basic_block != self.catch_block {
			self.catch_block.move_after(last_basic_block).unwrap();
		}

		self.builder.position_at_end(self.catch_block);

		let exception_type = context.struct_type(&[ptr_type.into(), i32_type.into()], false);

		let exception = self.builder.build_landing_pad(
			exception_type,
			self.functions.eh_personality,
			&[],
			true,
			"exception\0",
		)?;

		self.builder.build_call(
			self.functions.lifetime.end,
			&[tape_size.into(), self.pointers.tape.into()],
			"\0",
		)?;
		self.builder.build_call(
			self.functions.lifetime.end,
			&[i64_size.into(), self.pointers.pointer.into()],
			"\0",
		)?;

		self.builder.build_resume(exception)?;

		self.write_puts()?;

		self.debug_builder.di_builder.finalize();

		Ok(self.into_parts())
	}

	fn ops(&self, ops: &[BrainIr], mut op_count: usize) -> Result<(), AssemblyError> {
		for op in ops {
			let debug_loc = self.debug_builder.create_debug_location(
				self.context(),
				1,
				op_count as u32,
				self.functions
					.main
					.get_subprogram()
					.unwrap()
					.as_debug_info_scope(),
				None,
			);

			self.builder.set_current_debug_location(debug_loc);

			match op {
				BrainIr::Boundary => continue,
				BrainIr::MovePointer(offset) => self.move_pointer(*offset)?,
				BrainIr::SetCell(options) => {
					self.set_cell(options.value(), options.offset())?;
				}
				BrainIr::ChangeCell(options) => {
					self.change_cell(options.value(), options.offset())?;
				}
				BrainIr::SubCell(SubType::CellAt(options)) => self.sub_cell_at(*options)?,
				BrainIr::SubCell(SubType::FromCell(options)) => self.sub_from_cell(*options)?,
				BrainIr::DuplicateCell { values } => self.duplicate_cell(values)?,
				BrainIr::Output(options) => self.output(options)?,
				BrainIr::InputIntoCell => self.input_into_cell()?,
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops, op_count)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops, op_count)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset)?,
				BrainIr::MoveValueTo(options) => self.move_value_to(*options)?,
				BrainIr::CopyValueTo(options) => self.copy_value_to(*options)?,
				BrainIr::TakeValueTo(options) => self.take_value_to(*options)?,
				BrainIr::FetchValueFrom(options) => {
					self.fetch_value_from(*options)?;
				}
				BrainIr::ReplaceValueFrom(options) => {
					self.replace_value_from(*options)?;
				}
				BrainIr::ScaleValue(factor) => self.scale_value(*factor)?,
				BrainIr::SetRange(options) => self.set_range(options.value, options.range())?,
				BrainIr::SetManyCells(options) => {
					self.set_many_cells(&options.values, options.start.get_or_zero())?;
				}
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}

			op_count += 1;
		}

		Ok(())
	}

	fn into_parts(self) -> (Module<'ctx>, AssemblerFunctions<'ctx>, TargetMachine) {
		(self.module, self.functions, self.target_machine)
	}

	#[allow(clippy::unused_self)]
	fn setup_global_value<T>(&self, global: GlobalValue<'ctx>, initializer: &T)
	where
		T: BasicValue<'ctx>,
	{
		global.set_thread_local(false);
		global.set_unnamed_addr(true);
		global.set_linkage(Linkage::Private);
		global.set_initializer(initializer);
		global.set_constant(true);
	}
}

unsafe impl<'ctx> AsContextRef<'ctx> for InnerAssembler<'ctx> {
	fn as_ctx_ref(&self) -> LLVMContextRef {
		self.module.get_context().as_ctx_ref()
	}
}
