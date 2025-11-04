#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
	feature = "get_or_zero",
	feature(nonzero_internals),
	allow(internal_features)
)]
#![cfg_attr(
	any(feature = "contains_range", feature = "into_range"),
	feature(step_trait)
)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "contains_range")]
mod contains_range;
#[cfg(any(
	feature = "unwrap_from",
	feature = "get_or_zero",
	feature = "convert",
	feature = "into_range"
))]
mod convert;
#[cfg(feature = "insert_or_push")]
mod insert_or_push;
#[cfg(feature = "iter_ext")]
mod iter;
#[cfg(feature = "slice_ext")]
mod slice;

#[cfg(feature = "contains_range")]
pub use self::contains_range::*;
#[cfg(any(
	feature = "unwrap_from",
	feature = "get_or_zero",
	feature = "convert",
	feature = "into_range"
))]
pub use self::convert::*;
#[cfg(feature = "insert_or_push")]
pub use self::insert_or_push::*;
#[cfg(feature = "iter_ext")]
pub use self::iter::*;
#[cfg(feature = "slice_ext")]
pub use self::slice::*;
