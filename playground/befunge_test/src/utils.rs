use std::io::{self, prelude::*};

use super::CellInt;

#[must_use]
pub fn rand_nibble() -> u8 {
	rand::random::<u8>() & 3
}

#[must_use]
pub fn read_char() -> CellInt {
	let mut byte = [0u8];
	let mut stdin = io::stdin().lock();

	if stdin.read_exact(&mut byte).is_ok() {
		CellInt::from(byte[0])
	} else {
		-1
	}
}

#[must_use]
pub fn read_int() -> CellInt {
	let mut line = String::new();
	io::stdin()
		.lock()
		.read_line(&mut line)
		.expect("error reading number");

	line.trim().parse().unwrap_or_default()
}

pub fn put_char(n: CellInt) {
	let mut buf = [0u8; 4];
	let ch = std::char::from_u32(n as u32).unwrap_or(std::char::REPLACEMENT_CHARACTER);
	let s = ch.encode_utf8(&mut buf);
	_ = io::stdout().lock().write_all(s.as_bytes());
}

pub fn put_int(n: CellInt) {
	_ = write!(io::stdout().lock(), "{n} ");
}

pub fn pop(data: &[CellInt], stack_idx: &mut isize) -> CellInt {
	if *stack_idx < 0 {
		0
	} else {
		let v = data[*stack_idx as usize];
		*stack_idx = stack_idx.wrapping_sub(1);
		v
	}
}

pub fn push(data: &mut [CellInt], stack_idx: &mut isize, v: CellInt) {
	*stack_idx = stack_idx.wrapping_add(1);
	data[*stack_idx as usize] = v;
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn print_stack(stack: *const CellInt, stack_idx: isize) {
	eprintln!("{stack_idx}");
	let mut idx = 0;
	while idx <= stack_idx {
		eprint!(" {}", unsafe { *stack.offset(idx) });
		idx += 1;
	}

	eprintln!();
}
