#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
	io::{self, prelude::*},
	process::abort,
	slice,
};

#[unsafe(no_mangle)]
#[tracing::instrument(level = tracing::Level::TRACE)]
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
#[tracing::instrument(level = tracing::Level::TRACE, ret)]
pub unsafe extern "C" fn rust_getchar() -> u8 {
	unsafe { libc::getchar() as u8 }
}
