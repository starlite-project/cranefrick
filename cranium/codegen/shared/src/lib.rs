#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

pub mod const_hash;
pub mod constants;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
