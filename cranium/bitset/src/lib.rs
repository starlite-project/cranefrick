#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
mod compound;
mod scalar;

#[cfg(feature = "alloc")]
pub use self::compound::*;
pub use self::scalar::*;
