#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::ops::{Deref, DerefMut, Range};

use frick_operations::{BrainOperation, BrainOperationType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainInstruction {
	instr: BrainInstructionType,
	byte_offset: usize,
}

impl BrainInstruction {
	#[must_use]
	pub const fn new(instr: BrainInstructionType, byte_offset: usize) -> Self {
		Self { instr, byte_offset }
	}

	#[must_use]
	pub const fn instr(self) -> BrainInstructionType {
		self.instr
	}

	#[must_use]
	pub const fn byte_offset(self) -> usize {
		self.byte_offset
	}

	#[must_use]
	pub const fn span(self) -> Range<usize> {
		self.byte_offset()..self.byte_offset()
	}
}

impl Deref for BrainInstruction {
	type Target = BrainInstructionType;

	fn deref(&self) -> &Self::Target {
		&self.instr
	}
}

impl DerefMut for BrainInstruction {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.instr
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BrainInstructionType {
	LoadCellIntoRegister(Reg),
	StoreRegisterIntoCell(Reg),
	StoreImmediateIntoCell(u8),
	ChangeRegisterByImmediate(Reg, i8),
	InputIntoRegister(Reg),
	OutputFromRegister(Reg),
	LoadPointer,
	OffsetPointer(i32),
	StorePointer,
	StartLoop,
	EndLoop,
	JumpIfZero(Reg),
	JumpIfNotZero(Reg),
	NotImplemented,
}

pub trait ToInstructions {
	fn to_instructions(&self) -> Vec<BrainInstruction>;
}

impl ToInstructions for BrainOperation {
	fn to_instructions(&self) -> Vec<BrainInstruction> {
		match self.op() {
			&BrainOperationType::ChangeCell(value) => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::LoadCellIntoRegister(Reg(0)),
				BrainInstructionType::ChangeRegisterByImmediate(Reg(0), value),
				BrainInstructionType::StoreRegisterIntoCell(Reg(0)),
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::MovePointer(offset) => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::OffsetPointer(offset),
				BrainInstructionType::StorePointer,
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::InputIntoCell => [
				BrainInstructionType::InputIntoRegister(Reg(0)),
				BrainInstructionType::LoadPointer,
				BrainInstructionType::StoreRegisterIntoCell(Reg(0)),
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			&BrainOperationType::OutputCurrentCell => [
				BrainInstructionType::LoadPointer,
				BrainInstructionType::LoadCellIntoRegister(Reg(0)),
				BrainInstructionType::OutputFromRegister(Reg(0)),
			]
			.into_iter()
			.map(|x| BrainInstruction::new(x, self.span().start))
			.collect(),
			BrainOperationType::DynamicLoop(ops) => {
				let mut output = [
					BrainInstructionType::StartLoop,
					BrainInstructionType::LoadPointer,
					BrainInstructionType::LoadCellIntoRegister(Reg(0)),
					BrainInstructionType::JumpIfZero(Reg(0)),
				]
				.into_iter()
				.map(|x| BrainInstruction::new(x, self.span().start))
				.collect::<Vec<_>>();

				for op in ops {
					output.extend(op.to_instructions());
				}

				output.extend(
					[
						BrainInstructionType::LoadPointer,
						BrainInstructionType::LoadCellIntoRegister(Reg(0)),
						BrainInstructionType::JumpIfNotZero(Reg(0)),
						BrainInstructionType::EndLoop,
					]
					.into_iter()
					.map(|x| BrainInstruction::new(x, self.span().end)),
				);

				output
			}
			_ => vec![BrainInstruction::new(
				BrainInstructionType::NotImplemented,
				self.span().start,
			)],
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Reg(pub usize);

impl From<usize> for Reg {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl From<Reg> for usize {
	fn from(value: Reg) -> Self {
		value.0
	}
}
