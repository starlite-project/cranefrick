#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

pub mod ast;
pub mod codegen;
pub mod error;
pub mod files;
pub mod lexer;
pub mod sema;
pub mod stable_mapset;

macro_rules! declare_id {
    (
        $(#[$attr:meta])*
        $name:ident
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(pub usize);

        impl $name {
            #[must_use]
            pub const fn index(self) -> usize {
                self.0
            }
        }
    };
}

pub(crate) use declare_id;
