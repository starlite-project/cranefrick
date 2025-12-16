#![cfg_attr(docsrs, feature(doc_cfg))]

use llvm_sys::prelude::{LLVMContextRef, LLVMMetadataRef};

unsafe extern "C" {
	pub fn LLVMCreateDistinctNodeInContext(
		C: LLVMContextRef,
		Nodes: *mut LLVMMetadataRef,
		Count: ::libc::c_uint,
	) -> LLVMMetadataRef;

	pub fn LLVMCreateSelfReferentialDistinctNodeInContext(
		C: LLVMContextRef,
		Nodes: *mut LLVMMetadataRef,
		Count: ::libc::c_uint,
	) -> LLVMMetadataRef;
}
