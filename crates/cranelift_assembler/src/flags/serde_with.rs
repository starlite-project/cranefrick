use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::RegallocAlgorithm",
	rename_all = "snake_case"
)]
pub enum FakeRegallocAlgorithm {
	Backtracking,
}

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::OptLevel",
	rename_all = "snake_case"
)]
pub enum FakeOptLevel {
	None,
	Speed,
	SpeedAndSize,
}

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::TlsModel",
	rename_all = "snake_case"
)]
pub enum FakeTlsModel {
	None,
	ElfGd,
	Macho,
	Coff,
}

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::StackSwitchModel",
	rename_all = "snake_case"
)]
pub enum FakeStackSwitchModel {
	None,
	Basic,
	UpdateWindowsTib,
}

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::LibcallCallConv",
	rename_all = "snake_case"
)]
pub enum FakeLibcallCallConv {
	IsaDefault,
	Fast,
	Cold,
	SystemV,
	WindowsFastcall,
	AppleAarch64,
	Probestack,
}

#[derive(Serialize, Deserialize)]
#[serde(
	remote = "cranelift_codegen::settings::ProbestackStrategy",
	rename_all = "snake_case"
)]
pub enum FakeProbestackStrategy {
	Outline,
	Inline,
}
