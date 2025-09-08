use bitflags::bitflags;

bitflags! {
	pub struct SrcLoc: u32 {
		const CHANGE_CELL = 1 << 0;
		const MOVE_POINTER = 1 << 1;
		const SET_CELL = 1 << 2;
		const FIND_ZERO = 1 << 3;
		const INPUT_INTO_CELL = 1 << 4;
		const OUTPUT_CURRENT_CELL = 1 << 5;
		const OUTPUT_CHAR = 1 << 6;
		const MOVE_VALUE = 1 << 7;
		const TAKE_VALUE = 1 << 8;
		const FETCH_VALUE = 1 << 9;
		const DYNAMIC_LOOP = 1 << 10;
		const BLOCK = 1 << 11;
		const REPLACE_VALUE = 1 << 12;
		const OUTPUT_CHARS = 1 << 13;
		const SUB_CELL = 1 << 14;
		const SCALE_VALUE = 1 << 15;
		const SET_RANGE = 1 << 16;
		const CHANGE_RANGE = 1 << 17;
	}
}
