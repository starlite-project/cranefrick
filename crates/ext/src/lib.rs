#![cfg_attr(docsrs, feature(doc_cfg))]

use llvm_sys::prelude::{LLVMContextRef, LLVMValueRef};

unsafe extern "C" {
	pub fn LLVMMDDistinctNodeInContext2(
		C: LLVMContextRef,
		Vals: *mut LLVMValueRef,
		Count: ::libc::c_uint,
	) -> LLVMValueRef;
}
