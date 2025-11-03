mod debug_info;
mod functions;
mod load;
mod offset;
mod pointers;
mod sealed;

use frick_ir::{OffsetCellMarker, OffsetCellOptions, OffsetCellPrimitive};
use frick_utils::SliceExt as _;

pub use self::{debug_info::*, functions::*, load::*, offset::*, pointers::*};

pub fn is_contiguous<T: OffsetCellPrimitive, Marker: OffsetCellMarker>(
	values: &[OffsetCellOptions<T, Marker>],
) -> bool {
	values
		.windows_n::<2>()
		.all(|&[x, y]| x.offset().wrapping_add(1) == y.offset())
}
