#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	io::{self, prelude::*},
	slice,
};

unsafe extern "C" {
	fn rust_eh_personality(
		version: i32,
		actions: i32,
		exception_class: i64,
		exception_object: *mut u8,
		context: *mut u8,
	) -> i32;
}

#[unsafe(no_mangle)]
#[tracing::instrument]
pub unsafe extern "C" fn eh_personality(
	version: i32,
	actions: i32,
	exception_class: i64,
	exception_object: *mut u8,
	context: *mut u8,
) -> i32 {
	tracing::error!("exception raised, unwinding");

	unsafe { rust_eh_personality(version, actions, exception_class, exception_object, context) }
}

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C-unwind" fn putchar(c: libc::c_int) -> libc::c_int {
	let mut stdout = io::stdout().lock();

	let c_trunc = c as u8;

	stdout
		.write_all(slice::from_ref(&c_trunc))
		.and_then(|()| stdout.flush())
		.unwrap();

	c
}
