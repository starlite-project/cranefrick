use cranelift_codegen::{
	binemit::{Addend, CodeOffset, Reloc},
	entity::PrimaryMap,
	ir,
};
use serde::{Deserialize, Serialize};

use super::{ModuleReloc, ModuleRelocTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDescription {
	pub init: Init,
	pub function_decls: PrimaryMap<ir::FuncRef, ModuleRelocTarget>,
	pub data_decls: PrimaryMap<ir::GlobalValue, ModuleRelocTarget>,
	pub function_relocs: Vec<(CodeOffset, ir::FuncRef)>,
	pub data_relocs: Vec<(CodeOffset, ir::GlobalValue, Addend)>,
	pub custom_segment_section: Option<(String, String)>,
	pub align: Option<u64>,
	pub used: bool,
}

impl DataDescription {
	#[must_use]
	pub fn new() -> Self {
		Self {
			init: Init::Uninitialized,
			function_decls: PrimaryMap::new(),
			data_decls: PrimaryMap::new(),
			function_relocs: Vec::new(),
			data_relocs: Vec::new(),
			custom_segment_section: None,
			align: None,
			used: false,
		}
	}

	pub fn clear(&mut self) {
		self.init = Init::Uninitialized;
		self.function_decls.clear();
		self.data_decls.clear();
		self.function_relocs.clear();
		self.data_relocs.clear();
		self.custom_segment_section = None;
		self.align = None;
		self.used = false;
	}

	pub fn define_zeroinit(&mut self, size: usize) {
		debug_assert_eq!(self.init, Init::Uninitialized);
		self.init = Init::Zeros { size };
	}

	pub fn define(&mut self, contents: Box<[u8]>) {
		debug_assert_eq!(self.init, Init::Uninitialized);
		self.init = Init::Bytes { contents };
	}

	pub fn set_segment_section(&mut self, seg: String, sec: String) {
		self.custom_segment_section = Some((seg, sec));
	}

	pub fn set_align(&mut self, align: u64) {
		assert!(align.is_power_of_two());
		self.align = Some(align);
	}

	pub const fn set_used(&mut self, used: bool) {
		self.used = used;
	}

	pub fn import_function(&mut self, name: ModuleRelocTarget) -> ir::FuncRef {
		self.function_decls.push(name)
	}

	pub fn import_global_value(&mut self, name: ModuleRelocTarget) -> ir::GlobalValue {
		self.data_decls.push(name)
	}

	pub fn write_function_addr(&mut self, offset: CodeOffset, func: ir::FuncRef) {
		self.function_relocs.push((offset, func));
	}

	pub fn write_data_addr(&mut self, offset: CodeOffset, data: ir::GlobalValue, addend: Addend) {
		self.data_relocs.push((offset, data, addend));
	}

	pub fn all_relocs(&self, pointer_reloc: Reloc) -> impl Iterator<Item = ModuleReloc> + '_ {
		let func_relocs = self
			.function_relocs
			.iter()
			.map(move |&(offset, id)| ModuleReloc {
				kind: pointer_reloc,
				offset,
				name: self.function_decls[id].clone(),
				addend: 0,
			});

		let data_relocs = self
			.data_relocs
			.iter()
			.map(move |&(offset, id, addend)| ModuleReloc {
				kind: pointer_reloc,
				offset,
				name: self.data_decls[id].clone(),
				addend,
			});

		func_relocs.chain(data_relocs)
	}
}

impl Default for DataDescription {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Init {
	Uninitialized,
	Zeros { size: usize },
	Bytes { contents: Box<[u8]> },
}

impl Init {
	#[must_use]
	pub fn size(&self) -> usize {
		match self {
			Self::Uninitialized => panic!("data size not initialized yet"),
			Self::Zeros { size } => *size,
			Self::Bytes { contents } => contents.len(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{DataDescription, Init, ModuleRelocTarget};

	#[test]
	fn basic_data_context() {
		let mut data = DataDescription::new();
		assert_eq!(data.init, Init::Uninitialized);
		assert!(data.function_decls.is_empty());
		assert!(data.data_decls.is_empty());
		assert!(data.function_relocs.is_empty());
		assert!(data.data_relocs.is_empty());

		data.define_zeroinit(256);

		let _func_a = data.import_function(ModuleRelocTarget::user(0, 0));
		let func_b = data.import_function(ModuleRelocTarget::user(0, 1));
		let func_c = data.import_function(ModuleRelocTarget::user(0, 2));
		let _data_a = data.import_global_value(ModuleRelocTarget::user(0, 3));
		let data_b = data.import_global_value(ModuleRelocTarget::user(0, 4));

		data.write_function_addr(8, func_b);
		data.write_function_addr(16, func_c);
		data.write_data_addr(32, data_b, 27);

		assert_eq!(data.init, Init::Zeros { size: 256 });
		assert_eq!(data.function_decls.len(), 3);
		assert_eq!(data.data_decls.len(), 2);
		assert_eq!(data.function_relocs.len(), 2);
		assert_eq!(data.data_relocs.len(), 1);

		data.clear();

		assert_eq!(data.init, Init::Uninitialized);
		assert!(data.function_decls.is_empty());
		assert!(data.data_decls.is_empty());
		assert!(data.function_relocs.is_empty());
		assert!(data.data_relocs.is_empty());

		let contents = vec![33, 34, 35, 36];
		let contents_clone = contents.clone();
		data.define(contents.into_boxed_slice());

		assert_eq!(
			data.init,
			Init::Bytes {
				contents: contents_clone.into_boxed_slice()
			}
		);
		assert_eq!(data.function_decls.len(), 0);
		assert_eq!(data.data_decls.len(), 0);
		assert_eq!(data.function_relocs.len(), 0);
		assert_eq!(data.data_relocs.len(), 0);
	}
}
