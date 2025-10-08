#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	io::{self, prelude::*},
	slice,
};

pub use libc::getchar as rust_getchar;

unsafe extern "C" {
	pub fn rust_eh_personality(
		version: i32,
		actions: i32,
		exception_class: i64,
		exception_object: *mut u8,
		context: *mut u8,
	) -> i32;
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C-unwind" fn rust_putchar(c: libc::c_int) -> libc::c_int {
	let mut stdout = io::stdout().lock();

	let c_trunc = c as u8;

	stdout
		.write_all(slice::from_ref(&c_trunc))
		.and_then(|()| stdout.flush())
		.unwrap();

	c
}
