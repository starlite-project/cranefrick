use std::{
	fmt::{Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{
	Deserialize, Deserializer, Serialize, Serializer,
	de::{Error as DeError, IgnoredAny, MapAccess, SeqAccess, Visitor},
	ser::SerializeStruct as _,
};

use super::{ChangeCellMarker, ChangeCellOptions, ChangeCellPrimitive, Factor, Value};

impl<'de, T, Marker: ChangeCellMarker> Deserialize<'de> for ChangeCellOptions<T, Marker>
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
			ChangeCellOptionsVisitor {
				marker: PhantomData,
			},
		)
	}
}

// impl<T, Marker: ChangeCellMarker> Serialize for ChangeCellOptions<T, Marker>
// where
// 	T: ChangeCellPrimitive + Serialize,
// {
// 	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
// 		let mut state = serializer.serialize_struct("ChangeCellOptions", 2)?;

// 		state.serialize_field("value", &self.value)?;
// 		state.serialize_field("offset", &self.offset)?;

// 		state.end()
// 	}
// }

impl<T> Serialize for ChangeCellOptions<T, Factor>
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

impl<T> Serialize for ChangeCellOptions<T, Value>
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

struct ChangeCellOptionsVisitor<T: ChangeCellPrimitive, Marker: ChangeCellMarker> {
	marker: PhantomData<ChangeCellOptions<T, Marker>>,
}

impl<'de, T, Marker: ChangeCellMarker> Visitor<'de> for ChangeCellOptionsVisitor<T, Marker>
where
	T: ChangeCellPrimitive + Deserialize<'de>,
{
	type Value = ChangeCellOptions<T, Marker>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct ChangeCellOptions")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let Some(value) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				0,
				&"struct ChangeCellOptions with 2 elements",
			));
		};

		let Some(offset) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				1,
				&"struct ChangeCellOptions with 2 elements",
			));
		};

		Ok(ChangeCellOptions::new(value, offset))
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut value = None;
		let mut offset = None;

		while let Some(key) = map.next_key()? {
			match key {
				ChangeCellOptionsField::Value => {
					if value.is_some() {
						return Err(DeError::duplicate_field("value"));
					}

					value = Some(map.next_value()?);
				}
				ChangeCellOptionsField::Offset => {
					if offset.is_some() {
						return Err(DeError::duplicate_field("offset"));
					}

					offset = Some(map.next_value()?);
				}
				ChangeCellOptionsField::Ignore => {
					let _ = map.next_value::<IgnoredAny>()?;
				}
			}
		}

		let Some(value) = value else {
			return Err(DeError::missing_field("value"));
		};

		let Some(offset) = offset else {
			return Err(DeError::missing_field("offset"));
		};

		Ok(ChangeCellOptions::new(value, offset))
	}
}

struct ChangeCellOptionsFieldVisitor;

impl Visitor<'_> for ChangeCellOptionsFieldVisitor {
	type Value = ChangeCellOptionsField;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("field identifier")
	}

	fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
		Ok(match v {
			0 => ChangeCellOptionsField::Value,
			1 => ChangeCellOptionsField::Offset,
			_ => ChangeCellOptionsField::Ignore,
		})
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
		Ok(match v {
			"value" | "factor" => ChangeCellOptionsField::Value,
			"offset" => ChangeCellOptionsField::Offset,
			_ => ChangeCellOptionsField::Ignore,
		})
	}

	fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
		Ok(match v {
			b"value" | b"factor" => ChangeCellOptionsField::Value,
			b"offset" => ChangeCellOptionsField::Offset,
			_ => ChangeCellOptionsField::Ignore,
		})
	}
}

enum ChangeCellOptionsField {
	Value,
	Offset,
	Ignore,
}

impl<'de> Deserialize<'de> for ChangeCellOptionsField {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(ChangeCellOptionsFieldVisitor)
	}
}
