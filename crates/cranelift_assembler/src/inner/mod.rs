mod impls;

use std::{
	collections::HashMap,
	ops::{Deref, DerefMut},
};

use cranelift_codegen::ir::{
	AbiParam, FuncRef, Function, InstBuilder as _, SourceLoc, Type, Value, types,
};
use cranelift_frontend::{FuncInstBuilder, FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::JITModule;
use cranelift_module::{DataDescription, Linkage, Module};
use frick_assembler::{AssemblyError, TAPE_SIZE};
use frick_ir::BrainIr;
use frick_utils::GetOrZero as _;

use super::CraneliftAssemblyError;

pub struct InnerAssembler<'a> {
	builder: FunctionBuilder<'a>,
	read: FuncRef,
	write: FuncRef,
	ptr_variable: Variable,
	loads: HashMap<i32, Value>,
}

impl<'a> InnerAssembler<'a> {
	pub fn new(
		func: &'a mut Function,
		fn_ctx: &'a mut FunctionBuilderContext,
		module: &mut JITModule,
		ptr_type: Type,
	) -> Result<Self, CraneliftAssemblyError> {
		let tape_global_value = {
			let tape_id = module.declare_data("tape", Linkage::Local, true, false)?;

			let mut tape_description = DataDescription::new();

			tape_description.define_zeroinit(TAPE_SIZE);

			module.define_data(tape_id, &tape_description)?;

			module.declare_data_in_func(tape_id, func)
		};

		let mut builder = FunctionBuilder::new(func, fn_ctx);

		let block = builder.create_block();

		builder.switch_to_block(block);

		let ptr_variable = {
			let ptr_value = builder.declare_var(ptr_type);

			let tape_value = builder.ins().symbol_value(ptr_type, tape_global_value);

			builder.def_var(ptr_value, tape_value);

			ptr_value
		};

		let write = {
			let mut write_sig = module.make_signature();
			write_sig.params.push(AbiParam::new(types::I32));

			module.declare_function("write", Linkage::Import, &write_sig)
		}?;

		let read = {
			let mut read_sig = module.make_signature();
			read_sig.params.push(AbiParam::new(ptr_type));

			module.declare_function("read", Linkage::Import, &read_sig)
		}?;

		let write = module.declare_func_in_func(write, builder.func);
		let read = module.declare_func_in_func(read, builder.func);

		Ok(Self {
			builder,
			read,
			write,
			ptr_variable,
			loads: HashMap::new(),
		})
	}

	pub fn assemble(
		mut self,
		ops: &[BrainIr],
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		self.ops(ops, 0)?;

		self.ins().return_(&[]);

		self.seal_all_blocks();

		self.builder.finalize();

		Ok(())
	}

	fn ops(
		&mut self,
		ops: &[BrainIr],
		mut op_count: u32,
	) -> Result<(), AssemblyError<CraneliftAssemblyError>> {
		for op in ops {
			self.set_srcloc(SourceLoc::new(op_count));

			match op {
				BrainIr::Boundary => continue,
				BrainIr::MovePointer(offset) => self.move_pointer(*offset),
				BrainIr::SetCell(value, offset) => {
					self.set_cell(*value, offset.get_or_zero());
				}
				BrainIr::ChangeCell(value, offset) => {
					self.change_cell(*value, offset.get_or_zero());
				}
				BrainIr::Output(options) => self.output(options)?,
				BrainIr::InputIntoCell => self.input_into_cell(),
				BrainIr::DynamicLoop(ops) => self.dynamic_loop(ops, op_count)?,
				BrainIr::IfNotZero(ops) => self.if_not_zero(ops, op_count)?,
				BrainIr::FindZero(offset) => self.find_zero(*offset),
				BrainIr::MoveValueTo(options) => self.move_value_to(*options),
				BrainIr::TakeValueTo(options) => self.take_value_to(*options),
				BrainIr::FetchValueFrom(options) => self.fetch_value_from(*options),
				BrainIr::ReplaceValueFrom(options) => self.replace_value_from(*options),
				BrainIr::ScaleValue(factor) => self.scale_value(*factor),
				BrainIr::SetRange(options) => self.set_range(options.value, options.range()),
				_ => return Err(AssemblyError::NotImplemented(op.clone())),
			}

			op_count += 1;
		}

		Ok(())
	}

	fn ins<'short>(&'short mut self) -> FuncInstBuilder<'short, 'a> {
		self.builder.ins()
	}

	fn ptr_value(&mut self) -> Value {
		let variable = self.ptr_variable;

		self.use_var(variable)
	}

	fn const_u8(&mut self, value: u8) -> Value {
		self.ins().iconst(types::I8, i64::from(value))
	}
}

impl<'a> Deref for InnerAssembler<'a> {
	type Target = FunctionBuilder<'a>;

	fn deref(&self) -> &Self::Target {
		&self.builder
	}
}

impl DerefMut for InnerAssembler<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.builder
	}
}
