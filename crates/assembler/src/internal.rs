use cranelift_codegen::ir::{Function, Inst, Value, function::FunctionStencil};
use cranelift_frontend::FunctionBuilder;

use super::Assembler;

pub(crate) trait FunctionStencilExt {
	fn first_result(&self, inst: Inst) -> Value;
}

impl FunctionStencilExt for FunctionStencil {
	fn first_result(&self, inst: Inst) -> Value {
		self.dfg.first_result(inst)
	}
}

impl FunctionStencilExt for Function {
	fn first_result(&self, inst: Inst) -> Value {
		self.stencil.first_result(inst)
	}
}

impl FunctionStencilExt for FunctionBuilder<'_> {
	fn first_result(&self, inst: Inst) -> Value {
		self.func.first_result(inst)
	}
}

impl FunctionStencilExt for Assembler<'_> {
	fn first_result(&self, inst: Inst) -> Value {
		self.builder.first_result(inst)
	}
}
