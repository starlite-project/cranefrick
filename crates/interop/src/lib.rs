#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
	io::{self, prelude::*},
	slice,
};

// pub use libc::getchar as rust_getchar;

unsafe extern "Rust" {
	pub fn rust_eh_personality(
		version: i32,
		actions: i32,
		exception_class: i64,
		exception_object: *mut u8,
		context: *mut u8,
	) -> i32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_putchar(c: u8) {
	let mut stdout = io::stdout().lock();

	if stdout
		.write_all(slice::from_ref(&c))
		.and_then(|()| stdout.flush())
		.is_err()
	{
		std::process::abort();
	}
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C" fn rust_getchar() -> libc::c_int {
	unsafe { libc::getchar() }
}
