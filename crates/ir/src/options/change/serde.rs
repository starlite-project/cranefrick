use std::{
	fmt::{Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{
	Deserialize, Deserializer, Serialize, Serializer,
	de::{Error as DeError, IgnoredAny, MapAccess, SeqAccess, Visitor},
	ser::SerializeStruct as _,
};

use super::{
	ChangeCellMarker, ChangeCellOptions, ChangeCellPrimitive, Factor, FactoredChangeCellOptions,
	Value, ValuedChangeCellOptions,
};

impl<'de, T> Deserialize<'de> for FactoredChangeCellOptions<T>
where
	T: ChangeCellPrimitive + Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		const FIELDS: &[&str] = &["factor", "offset"];

		deserializer.deserialize_struct(
			"ChangeCellOptions",
			FIELDS,
			FactoredChangeCellOptionsVisitor {
				marker: PhantomData,
			},
		)
	}
}

impl<'de, T> Deserialize<'de> for ValuedChangeCellOptions<T>
where
	T: ChangeCellPrimitive + Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		const FIELDS: &[&str] = &["value", "offset"];

		deserializer.deserialize_struct(
			"ChangeCellOptions",
			FIELDS,
			ValuedChangeCellOptionsVisitor {
				marker: PhantomData,
			},
		)
	}
}

impl<T> Serialize for FactoredChangeCellOptions<T>
where
	T: ChangeCellPrimitive + Serialize,
{
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_struct("ChangeCellOptions", 2)?;

		state.serialize_field("factor", &self.factor())?;
		state.serialize_field("offset", &self.offset)?;

		state.end()
	}
}

impl<T> Serialize for ValuedChangeCellOptions<T>
where
	T: ChangeCellPrimitive + Serialize,
{
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_struct("ChangeCellOptions", 2)?;

		state.serialize_field("value", &self.value())?;
		state.serialize_field("offset", &self.offset)?;

		state.end()
	}
}

struct FactoredChangeCellOptionsVisitor<T: ChangeCellPrimitive> {
	marker: PhantomData<FactoredChangeCellOptions<T>>,
}

impl<'de, T> Visitor<'de> for FactoredChangeCellOptionsVisitor<T>
where
	T: ChangeCellPrimitive + Deserialize<'de>,
{
	type Value = FactoredChangeCellOptions<T>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct FactoredChangeCellOptions")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let Some(factor) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				0,
				&"struct FactoredChangeCellOptions with 2 elements",
			));
		};

		let Some(offset) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				1,
				&"struct FactoredChangeCellOptions with 2 elements",
			));
		};

		Ok(FactoredChangeCellOptions::new(factor, offset))
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut factor = None;
		let mut offset = None;

		while let Some(key) = map.next_key()? {
			match key {
				FactoredChangeCellOptionsField::Factor => {
					if factor.is_some() {
						return Err(DeError::duplicate_field("factor"));
					}

					factor = Some(map.next_value()?);
				}
				FactoredChangeCellOptionsField::Offset => {
					if offset.is_some() {
						return Err(DeError::duplicate_field("offset"));
					}

					offset = Some(map.next_value()?);
				}
				FactoredChangeCellOptionsField::Ignore => {
					let _ = map.next_value::<IgnoredAny>()?;
				}
			}
		}

		let Some(factor) = factor else {
			return Err(DeError::missing_field("factor"));
		};

		let Some(offset) = offset else {
			return Err(DeError::missing_field("offset"));
		};

		Ok(FactoredChangeCellOptions::new(factor, offset))
	}
}

struct ValuedChangeCellOptionsVisitor<T: ChangeCellPrimitive> {
	marker: PhantomData<ValuedChangeCellOptions<T>>,
}

impl<'de, T> Visitor<'de> for ValuedChangeCellOptionsVisitor<T>
where
	T: ChangeCellPrimitive + Deserialize<'de>,
{
	type Value = ValuedChangeCellOptions<T>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct ValuedChangeCellOptions")
	}
}

struct FactoredChangeCellOptionsFieldVisitor;

impl Visitor<'_> for FactoredChangeCellOptionsFieldVisitor {
	type Value = FactoredChangeCellOptionsField;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("field identifier")
	}

	fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
		Ok(match v {
			0 => FactoredChangeCellOptionsField::Factor,
			1 => FactoredChangeCellOptionsField::Offset,
			_ => FactoredChangeCellOptionsField::Ignore,
		})
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
		Ok(match v {
			"factor" => FactoredChangeCellOptionsField::Factor,
			"offset" => FactoredChangeCellOptionsField::Offset,
			_ => FactoredChangeCellOptionsField::Ignore,
		})
	}

	fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
		Ok(match v {
			b"factor" => FactoredChangeCellOptionsField::Factor,
			b"offset" => FactoredChangeCellOptionsField::Offset,
			_ => FactoredChangeCellOptionsField::Ignore,
		})
	}
}

struct ValuedChangeCellOptionsFieldVisitor;

impl Visitor<'_> for ValuedChangeCellOptionsFieldVisitor {
	type Value = ValuedChangeCellOptionsField;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("field identifier")
	}

	fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
		Ok(match v {
			0 => ValuedChangeCellOptionsField::Value,
			1 => ValuedChangeCellOptionsField::Offset,
			_ => ValuedChangeCellOptionsField::Ignore,
		})
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
		Ok(match v {
			"value" => ValuedChangeCellOptionsField::Value,
			"offset" => ValuedChangeCellOptionsField::Offset,
			_ => ValuedChangeCellOptionsField::Ignore,
		})
	}

	fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
		Ok(match v {
			b"value" => ValuedChangeCellOptionsField::Value,
			b"offset" => ValuedChangeCellOptionsField::Offset,
			_ => ValuedChangeCellOptionsField::Ignore,
		})
	}
}

enum FactoredChangeCellOptionsField {
	Factor,
	Offset,
	Ignore,
}

impl<'de> Deserialize<'de> for FactoredChangeCellOptionsField {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(FactoredChangeCellOptionsFieldVisitor)
	}
}

enum ValuedChangeCellOptionsField {
	Value,
	Offset,
	Ignore,
}

impl<'de> Deserialize<'de> for ValuedChangeCellOptionsField {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(ValuedChangeCellOptionsFieldVisitor)
	}
}
