mod debug_info;
mod functions;
mod load;
mod offset;
mod pointers;
mod sealed;

use frick_ir::{OffsetCellMarker, OffsetCellOptions, OffsetCellPrimitive};

pub use self::{debug_info::*, functions::*, load::*, offset::*, pointers::*};

pub fn is_contiguous<T: OffsetCellPrimitive, Marker: OffsetCellMarker>(
	values: &[OffsetCellOptions<T, Marker>],
) -> bool {
	values
		.windows(2)
		.all(|w| w[0].offset().wrapping_add(1) == w[1].offset())
}
