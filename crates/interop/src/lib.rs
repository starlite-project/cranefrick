#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	io::{self, prelude::*},
	process::abort,
	slice,
};

pub unsafe extern "C" fn putchar(c: u32) {
	if cfg!(target_os = "windows") && c >= 128 {
		return;
	}

	let mut stdout = io::stdout().lock();

	let value = {
		let Some(ch) = char::from_u32(c) else {
			return;
		};

		ch as u8
	};

	let result = stdout
		.write_all(slice::from_ref(&value))
		.and_then(|()| stdout.flush());

	if result.is_err() {
		abort()
	}
}

#[must_use]
pub unsafe extern "C" fn getchar() -> u32 {
	let mut stdin = io::stdin().lock();
	let c = loop {
		let mut value = 0;
		let err = stdin.read_exact(slice::from_mut(&mut value));

		if let Err(e) = err {
			if !matches!(e.kind(), io::ErrorKind::UnexpectedEof) {
				abort();
			}

			value = 0;
		}

		if cfg!(target_os = "windows") && matches!(value, b'\r') {
			continue;
		}

		break value as char;
	};

	c as u32
}
