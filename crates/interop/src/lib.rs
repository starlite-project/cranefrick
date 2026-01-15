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
	let mut stdin = io::stdin().lock();

	let mut value = 0;

	if let Err(e) = stdin.read_exact(slice::from_mut(&mut value)) {
		match e.kind() {
			io::ErrorKind::UnexpectedEof => value = 10,
			_ => abort(),
		}
	}

	value
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C" fn rust_alloc(size: usize) -> *mut c_void {
	unsafe { libc::malloc(size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_free(ptr: *mut c_void) {
	unsafe { libc::free(ptr) }
}
