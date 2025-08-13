#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::cmp;

use cranefrick_egraph_numeric_id::NumericId;

#[derive(Clone)]
#[repr(transparent)]
pub struct UnionFind<Value> {
	parents: Vec<Value>,
}
