#![allow(clippy::unused_self)]

mod const_fold;
mod dce;

pub use self::{const_fold::*, dce::*};
