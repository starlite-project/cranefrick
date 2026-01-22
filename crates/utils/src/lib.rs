#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
	feature = "nightly",
	feature(nonzero_internals, step_trait),
	allow(internal_features)
)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "tracing_indicatif_ext")]
#[cfg_attr(feature = "tracing_indicatif_ext", doc(hidden))]
pub use tracing_indicatif;

mod contains_range;
mod convert;
#[cfg(feature = "alloc")]
mod insert_or_push;
mod iter;
mod runtime_array;
mod slice;
#[cfg(feature = "tracing_indicatif_ext")]
mod tracing_indicatif_ext;

#[cfg(feature = "alloc")]
pub use self::insert_or_push::*;
pub use self::{contains_range::*, convert::*, iter::*, runtime_array::*, slice::*};
