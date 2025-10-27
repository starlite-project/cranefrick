#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
	feature = "get_or_zero",
	feature(nonzero_internals),
	allow(internal_features)
)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(any(feature = "unwrap_from", feature = "get_or_zero", feature = "convert"))]
mod convert;
#[cfg(feature = "insert_or_push")]
mod insert_or_push;
#[cfg(feature = "iter")]
mod iter;

#[cfg(any(feature = "unwrap_from", feature = "get_or_zero", feature = "convert"))]
pub use self::convert::*;
#[cfg(feature = "insert_or_push")]
pub use self::insert_or_push::*;
#[cfg(feature = "iter")]
pub use self::iter::*;
