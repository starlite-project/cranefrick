use alloc::vec::Vec;
use core::{
	fmt::{Debug, Display, Error as FmtError, Formatter, Result as FmtResult, Write as _},
	ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign},
	error::Error as CoreError,
};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::{Block, Instruction, InstructionRange};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct PhysicalRegister {
	bits: u8,
}

impl PhysicalRegister {
	pub const MAX: usize = (1 << Self::MAX_BITS) - 1;
	pub const MAX_BITS: usize = 6;
	pub const NUM_INDEX: usize = 1 << (Self::MAX_BITS + 2);

	#[must_use]
	pub const fn new(hw_enc: usize, class: RegisterClass) -> Self {
		debug_assert!(hw_enc <= Self::MAX);
		Self {
			bits: ((class as u8) << Self::MAX_BITS) | (hw_enc as u8),
		}
	}

	#[must_use]
	pub const fn hardware_encode(self) -> usize {
		self.bits as usize & Self::MAX
	}

	#[must_use]
	pub const fn class(self) -> RegisterClass {
		match (self.bits >> Self::MAX_BITS) & 0b11 {
			0 => RegisterClass::Int,
			1 => RegisterClass::Float,
			2 => RegisterClass::Vector,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn index(self) -> usize {
		self.bits as usize
	}

	#[must_use]
	pub const fn from_index(index: usize) -> Self {
		Self {
			bits: (index & (Self::NUM_INDEX - 1)) as u8,
		}
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self::new(Self::MAX, RegisterClass::Int)
	}
}

impl Debug for PhysicalRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("PhysicalRegister(hw = ")?;
		Display::fmt(&self.hardware_encode(), f)?;
		f.write_str(", class = ")?;
		Debug::fmt(&self.class(), f)?;
		f.write_str(", index = ")?;
		Display::fmt(&self.index(), f)?;
		f.write_char(')')
	}
}

impl Default for PhysicalRegister {
	fn default() -> Self {
		Self::invalid()
	}
}

impl Display for PhysicalRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('p')?;
		Display::fmt(&self.hardware_encode(), f)?;
		Display::fmt(&self.class(), f)
	}
}

impl From<usize> for PhysicalRegister {
	fn from(value: usize) -> Self {
		Self::from_index(value)
	}
}

impl From<PhysicalRegister> for usize {
	fn from(value: PhysicalRegister) -> Self {
		value.index()
	}
}

#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(transparent)]
pub struct PhysicalRegisterSet {
	bits: [Bits; Self::LEN],
}

impl PhysicalRegisterSet {
	const BITS: usize = core::mem::size_of::<Bits>() * 8;
	const LEN: usize = PhysicalRegister::NUM_INDEX.div_ceil(Self::BITS);

	#[must_use]
	pub const fn empty() -> Self {
		Self {
			bits: [0; Self::LEN],
		}
	}

	const fn split_index(reg: PhysicalRegister) -> (usize, usize) {
		let index = reg.index();
		(index >> Self::BITS.ilog2(), index & (Self::BITS - 1))
	}

	#[must_use]
	pub const fn contains(self, reg: PhysicalRegister) -> bool {
		let (index, bit) = Self::split_index(reg);
		!matches!(self.bits[index] & (1 << bit), 0)
	}

	#[must_use]
	pub const fn with(self, reg: PhysicalRegister) -> Self {
		let (index, bit) = Self::split_index(reg);
		let mut out = self;
		out.bits[index] |= 1 << bit;
		out
	}

	pub const fn add(&mut self, reg: PhysicalRegister) {
		let (index, bit) = Self::split_index(reg);
		self.bits[index] |= 1 << bit;
	}

	pub const fn remove(&mut self, reg: PhysicalRegister) {
		let (index, bit) = Self::split_index(reg);
		self.bits[index] &= !(1 << bit);
	}

	pub fn union_from(&mut self, other: Self) {
		*self |= other;
	}

	pub fn intersect_from(&mut self, other: Self) {
		*self &= other;
	}

	#[must_use]
	pub fn invert(self) -> Self {
		let mut set = self.bits;
		for (i, bit) in set.iter_mut().enumerate() {
			*bit = !self.bits[i];
		}

		Self { bits: set }
	}

	#[must_use]
	pub const fn is_empty(self, class: RegisterClass) -> bool {
		matches!(self.bits[class as usize], 0)
	}

	#[must_use]
	pub const fn iter(self) -> PhysicalRegisterSetIter {
		PhysicalRegisterSetIter {
			set: self,
			index: 0,
		}
	}
}

impl BitAnd for PhysicalRegisterSet {
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output {
		let mut out = self;
		out.bitand_assign(rhs);
		out
	}
}

impl BitAndAssign for PhysicalRegisterSet {
	fn bitand_assign(&mut self, rhs: Self) {
		for i in 0..self.bits.len() {
			self.bits[i] &= rhs.bits[i];
		}
	}
}

impl BitOr for PhysicalRegisterSet {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		let mut out = self;
		out.bitor_assign(rhs);
		out
	}
}

impl BitOrAssign for PhysicalRegisterSet {
	fn bitor_assign(&mut self, rhs: Self) {
		for i in 0..self.bits.len() {
			self.bits[i] |= rhs.bits[i];
		}
	}
}

impl Display for PhysicalRegisterSet {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('{')?;
		for reg in *self {
			Display::fmt(&reg, f)?;
			f.write_str(", ")?;
		}

		f.write_char('}')
	}
}

impl FromIterator<PhysicalRegister> for PhysicalRegisterSet {
	fn from_iter<T>(iter: T) -> Self
	where
		T: IntoIterator<Item = PhysicalRegister>,
	{
		let mut set = Self::default();
		for reg in iter {
			set.add(reg);
		}

		set
	}
}

impl IntoIterator for PhysicalRegisterSet {
	type IntoIter = PhysicalRegisterSetIter;
	type Item = PhysicalRegister;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct PhysicalRegisterSetIter {
	set: PhysicalRegisterSet,
	index: usize,
}

impl Iterator for PhysicalRegisterSetIter {
	type Item = PhysicalRegister;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let bits = self.set.bits.get_mut(self.index)?;
			if !matches!(bits, 0) {
				let bit = bits.trailing_zeros();
				*bits &= !(1 << bit);
				let index = bit as usize + self.index * PhysicalRegisterSet::BITS;
				break Some(PhysicalRegister::from_index(index));
			}
			self.index += 1;
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct VirtualRegister {
	bits: u32,
}

impl VirtualRegister {
	pub const MAX: usize = (1 << Self::MAX_BITS) - 1;
	pub const MAX_BITS: usize = 21;

	#[must_use]
	pub const fn new(virt_reg: usize, class: RegisterClass) -> Self {
		debug_assert!(virt_reg <= Self::MAX);
		Self {
			bits: ((virt_reg as u32) << 2) | (class as u8 as u32),
		}
	}

	#[must_use]
	pub const fn virtual_register(self) -> usize {
		(self.bits >> 2) as usize
	}

	#[must_use]
	pub const fn class(self) -> RegisterClass {
		match self.bits & 0b11 {
			0 => RegisterClass::Int,
			1 => RegisterClass::Float,
			2 => RegisterClass::Vector,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn bits(self) -> usize {
		self.bits as usize
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self::new(Self::MAX, RegisterClass::Int)
	}
}

impl Debug for VirtualRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("VirtualRegister(vreg = ")?;
		Display::fmt(&self.virtual_register(), f)?;
		f.write_str(", class = ")?;
		Debug::fmt(&self.class(), f)?;
		f.write_char(')')
	}
}

impl Display for VirtualRegister {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char('v')?;
		Display::fmt(&self.virtual_register(), f)
	}
}

impl From<u32> for VirtualRegister {
	fn from(value: u32) -> Self {
		Self { bits: value }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SpillSlot {
	bits: u32,
}

impl SpillSlot {
	pub const MAX: usize = (1 << 24) - 1;

	#[must_use]
	pub const fn new(slot: usize) -> Self {
		debug_assert!(slot <= Self::MAX);
		Self { bits: slot as u32 }
	}

	#[must_use]
	pub const fn index(self) -> usize {
		(self.bits & 0x00ff_ffff) as usize
	}

	#[must_use]
	pub const fn plus(self, offset: usize) -> Self {
		Self::new(self.index() + offset)
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self { bits: 0xffff_ffff }
	}

	#[must_use]
	pub const fn is_invalid(self) -> bool {
		matches!(self.bits, 0xffff_ffff)
	}

	#[must_use]
	pub const fn is_valid(self) -> bool {
		!self.is_invalid()
	}
}

impl Display for SpillSlot {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("stack")?;
		Display::fmt(&self.index(), f)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Operand {
	bits: u32,
}

impl Operand {
	#[must_use]
	pub fn new(
		vreg: VirtualRegister,
		constraint: OperandConstraint,
		ty: OperandType,
		pos: OperandPosition,
	) -> Self {
		let constraint_field = match constraint {
			OperandConstraint::Any => 0,
			OperandConstraint::Register => 1,
			OperandConstraint::Stack => 2,
			OperandConstraint::FixedRegister(preg) => {
				debug_assert_eq!(preg.class(), vreg.class());
				0b100_0000 | preg.hardware_encode() as u32
			}
			OperandConstraint::Reuse(which) => {
				debug_assert!(which <= 31);
				0b010_0000 | which as u32
			}
		};

		let class_field = u32::from(vreg.class() as u8);
		let pos_field = u32::from(pos as u8);
		let ty_field = u32::from(ty as u8);

		Self {
			bits: vreg.virtual_register() as u32
				| (class_field << 21)
				| (pos_field << 23)
				| (ty_field << 24)
				| (constraint_field << 25),
		}
	}

	#[must_use]
	pub fn use_register(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Register,
			OperandType::Use,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub fn use_register_at_end(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Register,
			OperandType::Use,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn def_register(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Register,
			OperandType::Def,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn def_register_at_start(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Register,
			OperandType::Def,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub fn temp_register(vreg: VirtualRegister) -> Self {
		Self::def_register_at_start(vreg)
	}

	#[must_use]
	pub fn def_reuse_register(vreg: VirtualRegister, idx: usize) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Reuse(idx),
			OperandType::Def,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn use_fixed_register(vreg: VirtualRegister, preg: PhysicalRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::FixedRegister(preg),
			OperandType::Use,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub fn def_fixed_register(vreg: VirtualRegister, preg: PhysicalRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::FixedRegister(preg),
			OperandType::Def,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn use_fixed_register_at_end(vreg: VirtualRegister, preg: PhysicalRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::FixedRegister(preg),
			OperandType::Use,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn def_fixed_register_at_start(vreg: VirtualRegister, preg: PhysicalRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::FixedRegister(preg),
			OperandType::Def,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub fn use_any(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Any,
			OperandType::Use,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub fn def_any(vreg: VirtualRegister) -> Self {
		Self::new(
			vreg,
			OperandConstraint::Any,
			OperandType::Def,
			OperandPosition::Late,
		)
	}

	#[must_use]
	pub fn fixed_nonallocatable(preg: PhysicalRegister) -> Self {
		Self::new(
			VirtualRegister::new(VirtualRegister::MAX, preg.class()),
			OperandConstraint::FixedRegister(preg),
			OperandType::Use,
			OperandPosition::Early,
		)
	}

	#[must_use]
	pub const fn virtual_register(self) -> VirtualRegister {
		let vreg_idx = (self.bits as usize) & VirtualRegister::MAX;
		VirtualRegister::new(vreg_idx, self.class())
	}

	#[must_use]
	pub const fn class(self) -> RegisterClass {
		let class_field = (self.bits >> 21) & 3;
		match class_field {
			0 => RegisterClass::Int,
			1 => RegisterClass::Float,
			2 => RegisterClass::Vector,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn ty(self) -> OperandType {
		let ty_field = (self.bits >> 24) & 1;
		match ty_field {
			0 => OperandType::Def,
			1 => OperandType::Use,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn position(self) -> OperandPosition {
		let pos_field = (self.bits >> 23) & 1;
		match pos_field {
			0 => OperandPosition::Early,
			1 => OperandPosition::Late,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn constraint(self) -> OperandConstraint {
		let constraint_field = ((self.bits >> 25) as usize) & 127;
		if !matches!(constraint_field & 0b100_0000, 0) {
			OperandConstraint::FixedRegister(PhysicalRegister::new(0b011_1111, self.class()))
		} else if !matches!(constraint_field & 0b010_0000, 0) {
			OperandConstraint::Reuse(constraint_field & 0b001_1111)
		} else {
			match constraint_field {
				0 => OperandConstraint::Any,
				1 => OperandConstraint::Register,
				2 => OperandConstraint::Stack,
				_ => unreachable!(),
			}
		}
	}

	#[must_use]
	pub const fn as_fixed_nonallocatable(self) -> Option<PhysicalRegister> {
		match self.constraint() {
			OperandConstraint::FixedRegister(preg)
				if matches!(
					self.virtual_register().virtual_register(),
					VirtualRegister::MAX
				) =>
			{
				Some(preg)
			}
			_ => None,
		}
	}

	#[must_use]
	pub const fn bits(self) -> u32 {
		self.bits
	}

	#[must_use]
	pub const fn from_bits(bits: u32) -> Self {
		debug_assert!(bits >> 29 <= 4);
		Self { bits }
	}
}

impl Debug for Operand {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self, f)
	}
}

impl Display for Operand {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		if let Some(preg) = self.as_fixed_nonallocatable() {
			f.write_str("Fixed: ")?;
			return Display::fmt(&preg, f);
		}

		match (self.ty(), self.position()) {
			(OperandType::Def, OperandPosition::Late)
			| (OperandType::Use, OperandPosition::Early) => {
				Debug::fmt(&self.ty(), f)?;
			}
			_ => {
				Debug::fmt(&self.ty(), f)?;
				f.write_char('@')?;
				Debug::fmt(&self.position(), f)?;
			}
		}

		f.write_str(": ")?;
		Display::fmt(&self.virtual_register(), f)?;
		Display::fmt(&self.class(), f)?;
		f.write_char(' ')?;
		Display::fmt(&self.constraint(), f)
	}
}

impl From<Operand> for u32 {
	fn from(value: Operand) -> Self {
		value.bits()
	}
}

impl From<u32> for Operand {
	fn from(value: u32) -> Self {
		Self::from_bits(value)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Allocation {
	bits: u32,
}

impl Allocation {
	pub(crate) const fn new(ty: AllocationType, index: usize) -> Self {
		debug_assert!(index < (1 << 28));
		Self {
			bits: ((ty as u8 as u32) << 29) | (index as u32),
		}
	}

	#[must_use]
	pub const fn none() -> Self {
		Self::new(AllocationType::None, 0)
	}

	#[must_use]
	pub const fn register(preg: PhysicalRegister) -> Self {
		Self::new(AllocationType::Register, preg.index())
	}

	#[must_use]
	pub const fn stack(slot: SpillSlot) -> Self {
		Self::new(AllocationType::Stack, slot.bits as usize)
	}

	#[must_use]
	pub const fn ty(self) -> AllocationType {
		match (self.bits >> 29) & 7 {
			0 => AllocationType::None,
			1 => AllocationType::Register,
			2 => AllocationType::Stack,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn is_none(self) -> bool {
		matches!(self.ty(), AllocationType::None)
	}

	#[must_use]
	pub const fn is_register(self) -> bool {
		matches!(self.ty(), AllocationType::Register)
	}

	#[must_use]
	pub const fn is_stack(self) -> bool {
		matches!(self.ty(), AllocationType::Stack)
	}

	#[must_use]
	pub const fn index(self) -> usize {
		(self.bits & ((1 << 28) - 1)) as usize
	}

	#[must_use]
	pub const fn as_register(self) -> Option<PhysicalRegister> {
		if self.is_register() {
			Some(PhysicalRegister::from_index(self.index()))
		} else {
			None
		}
	}

	#[must_use]
	pub const fn as_stack(self) -> Option<SpillSlot> {
		if self.is_stack() {
			Some(SpillSlot {
				bits: self.index() as u32,
			})
		} else {
			None
		}
	}

	#[must_use]
	pub const fn bits(self) -> u32 {
		self.bits
	}

	#[must_use]
	pub const fn from_bits(bits: u32) -> Self {
		debug_assert!(bits >> 29 >= 5);
		Self { bits }
	}
}

impl Debug for Allocation {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Display::fmt(&self, f)
	}
}

impl Display for Allocation {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self.ty() {
			AllocationType::None => f.write_str("none"),
			AllocationType::Register => Display::fmt(&self.as_register().ok_or(FmtError)?, f),
			AllocationType::Stack => Display::fmt(&self.as_stack().ok_or(FmtError)?, f),
		}
	}
}

impl From<Allocation> for u32 {
	fn from(value: Allocation) -> Self {
		value.bits()
	}
}

impl From<u32> for Allocation {
	fn from(value: u32) -> Self {
		Self::from_bits(value)
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ProgramPoint {
	bits: u32,
}

impl ProgramPoint {
	#[must_use]
	pub const fn new(inst: Instruction, pos: InstructionPosition) -> Self {
		let bits = (inst.0 << 1) | (pos as u8 as u32);
		Self { bits }
	}

	#[must_use]
	pub const fn before(inst: Instruction) -> Self {
		Self::new(inst, InstructionPosition::Before)
	}

	#[must_use]
	pub const fn after(inst: Instruction) -> Self {
		Self::new(inst, InstructionPosition::After)
	}

	#[must_use]
	pub const fn instruction(self) -> Instruction {
		Instruction::new(((self.bits as i32) >> 1) as usize)
	}

	#[must_use]
	pub fn position(self) -> InstructionPosition {
		match self.bits & 1 {
			0 => InstructionPosition::Before,
			1 => InstructionPosition::After,
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub const fn prev(self) -> Self {
		Self {
			bits: self.bits - 1,
		}
	}

	#[must_use]
	pub const fn to_index(self) -> u32 {
		self.bits
	}

	#[must_use]
	pub const fn from_index(index: u32) -> Self {
		Self { bits: index }
	}

	#[must_use]
	pub const fn invalid() -> Self {
		Self::before(Instruction::new(usize::MAX))
	}
}

impl Debug for ProgramPoint {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str("programpoint")?;
		Display::fmt(&self.instruction().index(), f)?;
		f.write_str(match self.position() {
			InstructionPosition::Before => "-pre",
			InstructionPosition::After => "-post",
		})
	}
}

impl From<ProgramPoint> for u32 {
	fn from(value: ProgramPoint) -> Self {
		value.to_index()
	}
}

impl From<u32> for ProgramPoint {
	fn from(value: u32) -> Self {
		Self::from_index(value)
	}
}

pub struct OutputIter<'a> {
	edits: &'a [(ProgramPoint, Edit)],
	instruction_range: InstructionRange,
}

impl<'a> Iterator for OutputIter<'a> {
	type Item = InstructionOrEdit<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.instruction_range.is_empty() {
			return None;
		}

		let next_inst = self.instruction_range.first();
		if let Some((edit, remaining_edits)) = self.edits.split_first()
			&& edit.0 <= ProgramPoint::before(next_inst)
		{
			self.edits = remaining_edits;
			return Some(InstructionOrEdit::Edit(&edit.1));
		}

		self.instruction_range = self.instruction_range.rest();
		Some(InstructionOrEdit::Instruction(next_inst))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineEnvironment {
	pub preferred_registers_by_class: [Vec<PhysicalRegister>; 3],
	pub non_preferred_registers_by_class: [Vec<PhysicalRegister>; 3],
	pub scratch_by_class: [Option<PhysicalRegister>; 3],
	pub fixed_stack_slots: Vec<PhysicalRegister>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Output {
	pub spillslots_count: usize,
	pub edits: Vec<(ProgramPoint, Edit)>,
	pub allocs: Vec<Allocation>,
	pub instruction_alloc_offsets: Vec<u32>,
	pub debug_locations: Vec<(u32, ProgramPoint, ProgramPoint, Allocation)>,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum RegisterClass {
	Int = 0,
	Float = 1,
	Vector = 2,
}

impl Display for RegisterClass {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_char(match self {
			Self::Int => 'i',
			Self::Float => 'f',
			Self::Vector => 'v',
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperandConstraint {
	Any,
	Register,
	Stack,
	FixedRegister(PhysicalRegister),
	Reuse(usize),
}

impl Display for OperandConstraint {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		match self {
			Self::Any => f.write_str("any"),
			Self::Register => f.write_str("register"),
			Self::Stack => f.write_str("stack"),
			Self::FixedRegister(preg) => {
				f.write_str("fixed(")?;
				Display::fmt(&preg, f)?;
				f.write_char(')')
			}
			Self::Reuse(idx) => {
				f.write_str("reuse(")?;
				Display::fmt(&idx, f)?;
				f.write_char(')')
			}
		}
	}
}

impl From<PhysicalRegister> for OperandConstraint {
	fn from(value: PhysicalRegister) -> Self {
		Self::FixedRegister(value)
	}
}

impl From<usize> for OperandConstraint {
	fn from(value: usize) -> Self {
		Self::Reuse(value)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OperandType {
	Def = 0,
	Use = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OperandPosition {
	Early = 0,
	Late = 1,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum AllocationType {
	None = 0,
	Register = 1,
	Stack = 2,
}

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum InstructionPosition {
	Before = 0,
	After = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Edit {
	Move { from: Allocation, to: Allocation },
}

#[derive(Debug, Clone)]
pub enum InstructionOrEdit<'a> {
	Instruction(Instruction),
	Edit(&'a Edit),
}

#[derive(Debug, Clone)]
#[expect(clippy::upper_case_acronyms)]
pub enum RegisterAllocError {
	CritEdge(Block, Block),
	SSA(VirtualRegister, Instruction),
	BB(Block),
	Branch(Instruction),
	EntryLivein,
	DisallowedBranchArg(Instruction),
	TooManyLiveRegisters,
	TooManyOperands,
}

impl Display for RegisterAllocError {
	fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.write_str(match self {
			Self::CritEdge(..) => "critical edge is not split between given blocks",
			Self::SSA(..) => "invalid SSA for given virtual register at given instruction",
			Self::BB(..) => "invalid basic block",
			Self::Branch(..) => "invalid branch",
			Self::EntryLivein => "a vreg is live-in on entry",
			Self::DisallowedBranchArg(..) => "a branch has non-blockparam arg(s) and at least one of the successor blocks has more than one predecessor",
			Self::TooManyLiveRegisters => "too many pinned vregs",
			Self::TooManyOperands => "too many operands on a single instruction"
		})
	}
}

impl CoreError for RegisterAllocError {}

pub trait Function {
	fn instruction_count(&self) -> usize;

	fn block_count(&self) -> usize;

	fn entry_block(&self) -> Block;

	fn block_instructions(&self, block: Block) -> InstructionRange;

	fn block_successors(&self, block: Block) -> &[Block];

	fn block_predecessors(&self, block: Block) -> &[Block];

	fn block_parameters(&self, block: Block) -> &[VirtualRegister];

	fn is_return(&self, inst: Instruction) -> bool;

	fn is_branch(&self, inst: Instruction) -> bool;

	fn branch_block_parameters(
		&self,
		block: Block,
		inst: Instruction,
		succ_idx: usize,
	) -> &[VirtualRegister];

	fn instruction_operands(&self, inst: Instruction) -> &[Operand];

	fn instruction_clobbers(&self, inst: Instruction) -> PhysicalRegisterSet;

	fn virtual_register_count(&self) -> usize;

	fn spillslot_size(&self, class: RegisterClass) -> usize;

	fn multi_spillslot_named_by_last_slot(&self) -> bool {
		false
	}

	fn debug_value_labels(&self) -> &[(VirtualRegister, Instruction, Instruction, u32)] {
		&[]
	}

	fn allow_multiple_virtual_register_defs(&self) -> bool {
		false
	}
}

type Bits = u64;
