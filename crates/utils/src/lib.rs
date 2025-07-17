#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "insert_or_push")]
mod insert_or_push;
#[cfg(feature = "shortname")]
mod shortname;

#[cfg(feature = "insert_or_push")]
pub use self::insert_or_push::*;
#[cfg(feature = "shortname")]
pub use self::shortname::*;
