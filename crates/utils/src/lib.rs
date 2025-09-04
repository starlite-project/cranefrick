#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![cfg_attr(
	feature = "get_or_zero",
	feature(nonzero_internals),
	allow(internal_features)
)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "convert")]
mod convert;
#[cfg(feature = "get_or_zero")]
mod get_or_zero;
#[cfg(feature = "insert_or_push")]
mod insert_or_push;
#[cfg(feature = "iter")]
mod iter;

#[cfg(feature = "convert")]
pub use self::convert::*;
#[cfg(feature = "get_or_zero")]
pub use self::get_or_zero::*;
#[cfg(feature = "insert_or_push")]
pub use self::insert_or_push::*;
#[cfg(feature = "iter")]
pub use self::iter::*;
