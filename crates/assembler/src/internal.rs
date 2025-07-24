use cranelift_codegen::ir::{
	ExceptionTable, ExceptionTableData, Function, function::FunctionStencil,
};
use cranelift_frontend::FunctionBuilder;

use super::Assembler;

pub trait FunctionExt {
	fn create_exception_table(&mut self, data: ExceptionTableData) -> ExceptionTable;
}

impl FunctionExt for FunctionStencil {
	fn create_exception_table(&mut self, data: ExceptionTableData) -> ExceptionTable {
		self.dfg.exception_tables.push(data)
	}
}

impl FunctionExt for Function {
	fn create_exception_table(&mut self, data: ExceptionTableData) -> ExceptionTable {
		self.stencil.create_exception_table(data)
	}
}

impl<'a> FunctionExt for FunctionBuilder<'a> {
	fn create_exception_table(&mut self, data: ExceptionTableData) -> ExceptionTable {
		self.func.create_exception_table(data)
	}
}

impl FunctionExt for Assembler<'_> {
	fn create_exception_table(&mut self, data: ExceptionTableData) -> ExceptionTable {
		self.builder.create_exception_table(data)
	}
}
