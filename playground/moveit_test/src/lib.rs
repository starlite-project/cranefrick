#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![no_std]

extern crate alloc;

mod drop_flag;
mod move_ref;
mod new;
mod slot;

pub use self::{drop_flag::*, move_ref::*, new::*, slot::*};
