use cranelift_codegen::settings::{
	LibcallCallConv, OptLevel, ProbestackStrategy, RegallocAlgorithm, StackSwitchModel, TlsModel,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "RegallocAlgorithm", rename_all = "snake_case")]
pub enum FakeRegallocAlgorithm {
	Backtracking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "OptLevel", rename_all = "snake_case")]
pub enum FakeOptLevel {
	None,
	Speed,
	SpeedAndSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "TlsModel", rename_all = "snake_case")]
pub enum FakeTlsModel {
	None,
	ElfGd,
	Macho,
	Coff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "StackSwitchModel", rename_all = "snake_case")]
pub enum FakeStackSwitchModel {
	None,
	Basic,
	UpdateWindowsTib,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "LibcallCallConv", rename_all = "snake_case")]
pub enum FakeLibcallCallConv {
	IsaDefault,
	Fast,
	Cold,
	SystemV,
	WindowsFastcall,
	AppleAarch64,
	Probestack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(remote = "ProbestackStrategy", rename_all = "snake_case")]
pub enum FakeProbestackStrategy {
	Outline,
	Inline,
}
