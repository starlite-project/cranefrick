use std::sync::{Arc, LazyLock};

use hugr::{
	Extension,
	extension::{ExtensionId, Version, prelude::*},
	ops::{ExtensionOp, OpName},
	types::{FuncValueType, PolyFuncTypeRV},
};

const ID: ExtensionId = ExtensionId::new_unchecked("bf");

const VERSION: Version = Version::new(0, 0, 1);

static EXTENSION: LazyLock<Arc<Extension>> = LazyLock::new(extension);

fn extension() -> Arc<Extension> {
	Extension::new_arc(ID, VERSION, |ext, extension_ref| {})
}

fn get_op(op_name: &str) -> ExtensionOp {
	EXTENSION
		.instantiate_extension_op(op_name, [])
		.unwrap()
		.into()
}

pub fn inc_op() -> ExtensionOp {
	get_op("INC")
}
