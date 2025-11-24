#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
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
pub unsafe extern "C" fn rust_getchar() -> libc::c_int {
	unsafe { libc::getchar() }
}
