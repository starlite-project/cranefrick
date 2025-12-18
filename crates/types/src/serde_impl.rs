use core::{
	fmt::{Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{
	de::{
		Deserialize, Deserializer, EnumAccess, Error as DeError, SeqAccess, Unexpected,
		VariantAccess as _, Visitor,
	},
	ser::{Serialize, Serializer},
};

use super::{RegOrImm, Register, RegisterType};

impl<'de, T> Deserialize<'de> for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_newtype_struct("Register", RegisterVisitor(PhantomData))
	}
}

impl<T> Serialize for Register<T>
where
	T: ?Sized + RegisterType,
{
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_newtype_struct("Register", &self.index())
	}
}

impl<'de, T: RegisterType> Deserialize<'de> for RegOrImm<T>
where
	T::RustType: Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_enum(
			"RegOrImm",
			REG_OR_IMM_VARIANTS,
			RegOrImmVisitor(PhantomData),
		)
	}
}

impl<T: RegisterType> Serialize for RegOrImm<T>
where
	T::RustType: Serialize,
{
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		match self {
			Self::Reg(r) => serializer.serialize_newtype_variant("RegOrImm", 0, "Reg", &r),
			Self::Imm(i) => serializer.serialize_newtype_variant("RegOrImm", 1, "Imm", &i),
		}
	}
}

#[repr(transparent)]
struct RegisterVisitor<T>(PhantomData<T>)
where
	T: ?Sized + RegisterType;

impl<'de, T> Visitor<'de> for RegisterVisitor<T>
where
	T: ?Sized + RegisterType,
{
	type Value = Register<T>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct Register")
	}

	fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		Deserialize::deserialize(deserializer).map(Register::<T>::new)
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let Some(index) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				0,
				&"struct Register with 1 element",
			));
		};

		Ok(Register::<T>::new(index))
	}
}

const REG_OR_IMM_VARIANTS: &[&str] = &["Reg", "Imm"];

struct RegOrImmVisitor<T: RegisterType>(PhantomData<RegOrImm<T>>);

impl<'de, T: RegisterType> Visitor<'de> for RegOrImmVisitor<T>
where
	T::RustType: Deserialize<'de>,
{
	type Value = RegOrImm<T>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("enum RegOrImm")
	}

	fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
	where
		A: EnumAccess<'de>,
	{
		match data.variant()? {
			(RegOrImmVariant::Reg, variant) => {
				variant.newtype_variant::<Register<T>>().map(RegOrImm::reg)
			}
			(RegOrImmVariant::Imm, variant) => {
				variant.newtype_variant::<T::RustType>().map(RegOrImm::imm)
			}
		}
	}
}

struct RegOrImmVariantVisitor;

impl Visitor<'_> for RegOrImmVariantVisitor {
	type Value = RegOrImmVariant;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("variant identifier")
	}

	fn visit_u64<E: DeError>(self, v: u64) -> Result<Self::Value, E> {
		match v {
			0 => Ok(RegOrImmVariant::Reg),
			1 => Ok(RegOrImmVariant::Imm),
			_ => Err(DeError::invalid_value(
				Unexpected::Unsigned(v),
				&"variant index 0 <= i < 2",
			)),
		}
	}

	fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
		match v {
			"Reg" => Ok(RegOrImmVariant::Reg),
			"Imm" => Ok(RegOrImmVariant::Imm),
			_ => Err(DeError::unknown_variant(v, REG_OR_IMM_VARIANTS)),
		}
	}
}

enum RegOrImmVariant {
	Reg,
	Imm,
}

impl<'de> Deserialize<'de> for RegOrImmVariant {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(RegOrImmVariantVisitor)
	}
}
