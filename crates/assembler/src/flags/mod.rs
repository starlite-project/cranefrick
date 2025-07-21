mod serde_with;

use cranelift_codegen::settings::{
	Flags, LibcallCallConv, OptLevel, ProbestackStrategy, RegallocAlgorithm, StackSwitchModel,
	TlsModel,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct AssemblerFlags {
	#[serde(with = "self::serde_with::FakeRegallocAlgorithm")]
	pub regalloc_algorithm: RegallocAlgorithm,
	#[serde(with = "self::serde_with::FakeOptLevel")]
	pub opt_level: OptLevel,
	#[serde(with = "self::serde_with::FakeTlsModel")]
	pub tls_model: TlsModel,
	#[serde(with = "self::serde_with::FakeStackSwitchModel")]
	pub stack_switch_model: StackSwitchModel,
	#[serde(with = "self::serde_with::FakeLibcallCallConv")]
	pub libcall_call_conv: LibcallCallConv,
	pub probestack_size_log2: u8,
	#[serde(with = "self::serde_with::FakeProbestackStrategy")]
	pub probestack_strategy: ProbestackStrategy,
	pub bb_padding_log2_minus_one: u8,
	pub log2_min_function_alignment: u8,
	pub regalloc_checker: bool,
	pub regalloc_verbose_logs: bool,
	pub enable_alias_analysis: bool,
	pub enable_verifier: bool,
	pub is_pic: bool,
	pub use_colocated_libcalls: bool,
	pub enable_float: bool,
	pub enable_nan_canonicalization: bool,
	pub enable_atomics: bool,
	pub enable_safepoints: bool,
	pub enable_llvm_abi_extensions: bool,
	pub enable_multi_ret_implicit_sret: bool,
	pub unwind_info: bool,
	pub preserve_frame_pointers: bool,
	pub machine_code_cfg_info: bool,
	pub enable_probestack: bool,
	pub enable_jump_tables: bool,
	pub enable_heap_access_spectre_mitigation: bool,
	pub enable_table_access_spectre_mitigation: bool,
	pub enable_incremental_compilation_cache_checks: bool,
}

impl AssemblerFlags {
	#[must_use]
	pub const fn regalloc_algorithm(self) -> &'static str {
		match self.regalloc_algorithm {
			RegallocAlgorithm::Backtracking => "backtracking",
		}
	}

	#[must_use]
	pub const fn opt_level(self) -> &'static str {
		match self.opt_level {
			OptLevel::None => "none",
			OptLevel::Speed => "speed",
			OptLevel::SpeedAndSize => "speed_and_size",
		}
	}

	#[must_use]
	pub const fn tls_model(self) -> &'static str {
		match self.tls_model {
			TlsModel::None => "none",
			TlsModel::Coff => "coff",
			TlsModel::ElfGd => "elf_gd",
			TlsModel::Macho => "macho",
		}
	}

	#[must_use]
	pub const fn libcall_call_conv(self) -> &'static str {
		match self.libcall_call_conv {
			LibcallCallConv::IsaDefault => "isa_default",
			LibcallCallConv::Cold => "cold",
			LibcallCallConv::Fast => "fast",
			LibcallCallConv::AppleAarch64 => "apple_aarch64",
			LibcallCallConv::Probestack => "probestack",
			LibcallCallConv::SystemV => "system_v",
			LibcallCallConv::WindowsFastcall => "windows_fastcall",
		}
	}

	#[must_use]
	pub const fn probestack_strategy(self) -> &'static str {
		match self.probestack_strategy {
			ProbestackStrategy::Inline => "inline",
			ProbestackStrategy::Outline => "outline",
		}
	}
}

impl Default for AssemblerFlags {
	fn default() -> Self {
		let flags = Flags::new(cranelift_codegen::settings::builder());

		Self::from(flags)
	}
}

impl From<Flags> for AssemblerFlags {
	fn from(value: Flags) -> Self {
		Self {
			regalloc_algorithm: value.regalloc_algorithm(),
			opt_level: value.opt_level(),
			tls_model: value.tls_model(),
			stack_switch_model: value.stack_switch_model(),
			libcall_call_conv: value.libcall_call_conv(),
			probestack_size_log2: value.probestack_size_log2(),
			probestack_strategy: value.probestack_strategy(),
			bb_padding_log2_minus_one: value.bb_padding_log2_minus_one(),
			log2_min_function_alignment: value.log2_min_function_alignment(),
			regalloc_checker: value.regalloc_checker(),
			regalloc_verbose_logs: value.regalloc_verbose_logs(),
			enable_alias_analysis: value.enable_alias_analysis(),
			enable_verifier: value.enable_verifier(),
			enable_atomics: value.enable_atomics(),
			enable_float: value.enable_float(),
			enable_heap_access_spectre_mitigation: value.enable_heap_access_spectre_mitigation(),
			enable_incremental_compilation_cache_checks: value
				.enable_incremental_compilation_cache_checks(),
			enable_jump_tables: value.enable_jump_tables(),
			is_pic: value.is_pic(),
			enable_llvm_abi_extensions: value.enable_llvm_abi_extensions(),
			enable_multi_ret_implicit_sret: value.enable_multi_ret_implicit_sret(),
			enable_nan_canonicalization: value.enable_nan_canonicalization(),
			enable_probestack: value.enable_probestack(),
			enable_safepoints: value.enable_safepoints(),
			enable_table_access_spectre_mitigation: value.enable_table_access_spectre_mitigation(),
			machine_code_cfg_info: value.machine_code_cfg_info(),
			preserve_frame_pointers: value.preserve_frame_pointers(),
			unwind_info: value.unwind_info(),
			use_colocated_libcalls: value.use_colocated_libcalls(),
		}
	}
}
