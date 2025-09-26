#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use core::ffi::CStr;

#[unsafe(no_mangle)]
#[must_use]
pub unsafe extern "C" fn puts(ptr: *const libc::c_char) -> i32 {
	let mut last = 0;

	let s = unsafe { CStr::from_ptr(ptr) };

	for c in s.to_bytes().iter().copied() {
		last = unsafe { libc::putchar(c.into()) };
	}

	last
}
