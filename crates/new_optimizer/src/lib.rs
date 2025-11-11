#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

mod inner;

use alloc::vec::Vec;

use frick_operations::BrainOperation;
use frick_utils::IntoIteratorExt as _;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Optimizer {
	inner: Vec<BrainOperation>,
}

impl Optimizer {
	pub fn new(ops: impl IntoIterator<Item = BrainOperation>) -> Self {
		Self {
			inner: ops.collect_to(),
		}
	}

	pub fn run(&mut self) {}
}
