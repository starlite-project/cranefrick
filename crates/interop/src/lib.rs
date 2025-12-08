#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
	ffi::c_void,
	io::{self, prelude::*},
	process::abort,
	slice,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_putchar(c: u8) {
	let mut stdout = io::stdout().lock();

	if stdout
		.write_all(slice::from_ref(&c))
		.and_then(|()| stdout.flush())
		.is_err()
	{
		abort();
	}
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C" fn rust_getchar() -> u8 {
	unsafe { libc::getchar() as u8 }
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C" fn rust_alloc(size: usize) -> *mut c_void {
	let ptr = unsafe { libc::malloc(size) };

	unsafe { core::ptr::write_bytes(ptr.cast::<u8>(), 0, size) }

	ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_free(ptr: *mut c_void) {
	unsafe { libc::free(ptr) }
}
