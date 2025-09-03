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

fn one_qb_func() -> PolyFuncTypeRV {
	FuncValueType::new_endo(vec![qb_t()]).into()
}

fn two_qb_func() -> PolyFuncTypeRV {
	FuncValueType::new_endo(vec![qb_t(), qb_t()]).into()
}

fn extension() -> Arc<Extension> {
	Extension::new_arc(ID, VERSION, |ext, extension_ref| {
		ext.add_op(
			OpName::new_inline("H"),
			"Hadamard".into(),
			one_qb_func(),
			extension_ref,
		)
		.unwrap();

		ext.add_op(
			OpName::new_inline("CX"),
			"CX".into(),
			two_qb_func(),
			extension_ref,
		)
		.unwrap();

		ext.add_op(
			OpName::new_inline("Measure"),
			"Measure a qubit, returning the qubit and the measurement result".into(),
			FuncValueType::new(vec![qb_t()], vec![qb_t(), bool_t()]),
			extension_ref,
		)
		.unwrap();
	})
}

fn get_op(op_name: &str) -> ExtensionOp {
	EXTENSION
		.instantiate_extension_op(op_name, [])
		.unwrap()
		.into()
}

pub fn h_gate() -> ExtensionOp {
	get_op("H")
}

pub fn cx_gate() -> ExtensionOp {
	get_op("CX")
}

pub fn measure() -> ExtensionOp {
	get_op("Measure")
}
