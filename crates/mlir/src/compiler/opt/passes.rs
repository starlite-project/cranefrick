#![allow(clippy::trivially_copy_pass_by_ref)]

use alloc::vec::Vec;

use super::Change;
use crate::BrainMlir;

pub fn combine_instructions(ops: &[BrainMlir; 2]) -> Option<Change> {
	match ops {
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] if *i1 == -i2 => {
			Some(Change::remove())
		}
		[BrainMlir::ChangeCell(i1), BrainMlir::ChangeCell(i2)] => Some(Change::replace(
			BrainMlir::change_cell(i1.wrapping_add(*i2)),
		)),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] if *i1 == -i2 => Some(Change::remove()),
		[BrainMlir::MovePtr(i1), BrainMlir::MovePtr(i2)] => {
			Some(Change::replace(BrainMlir::move_ptr(i1.wrapping_add(*i2))))
		}
		[BrainMlir::SetCell(i1), BrainMlir::ChangeCell(i2)] => {
			Some(Change::replace(BrainMlir::set_cell(i1.wrapping_add(*i2))))
		}
		[BrainMlir::SetCell(_), BrainMlir::SetCell(_)] => Some(Change::remove_offset(0)),
		_ => None,
	}
}

pub const fn clear_cell(ops: &[BrainMlir; 3]) -> Option<Change> {
	match ops {
		[
			BrainMlir::JumpRight,
			BrainMlir::ChangeCell(-1),
			BrainMlir::JumpLeft,
		] => Some(Change::replace(BrainMlir::set_cell(0))),
		_ => None,
	}
}

pub fn optimize_loops(program: &mut Vec<BrainMlir>) -> bool {
	let mut current_stack = Vec::new();
	let mut loop_stack = 0usize;
	let mut loop_start = 0usize;

	for (i, op) in program.iter().enumerate() {
		if matches!(loop_stack, 0) {
			if let Some(instr) = match op {
				BrainMlir::JumpLeft => unreachable!(),
				BrainMlir::JumpRight => {
					loop_start = i;
					loop_stack += 1;
					None
				}
				i => Some(i.clone()),
			} {
				current_stack.push(instr);
			}
		} else {
			match op {
				BrainMlir::JumpRight => loop_stack += 1,
				BrainMlir::JumpLeft => {
					loop_stack -= 1;
					if matches!(loop_stack, 0) {
						current_stack.push(BrainMlir::dynamic_loop(
							// program[loop_start + 1..i].iter().cloned(),
							{
								let mut s = program[loop_start + 1..i].to_vec();
								optimize_loops(&mut s);
								s
							}
						));
					}
				}
				_ => {}
			}
		}
	}

	let is_changed = *program != current_stack;

	if is_changed {
		_ = core::mem::replace(program, current_stack);
		true
	} else {
		false
	}
}
