use core::{
	fmt::{Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{
	Deserialize, Deserializer, Serialize, Serializer,
	de::{Error as DeError, SeqAccess, Visitor},
	ser::SerializeSeq,
};

use super::{EntityRef, SecondaryMap};

impl<'de, K: EntityRef, V> Deserialize<'de> for SecondaryMap<K, V>
where
	V: Clone + Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_seq(SecondaryMapVisitor {
			marker: PhantomData,
		})
	}
}

impl<K: EntityRef, V> Serialize for SecondaryMap<K, V>
where
	V: Clone + PartialEq + Serialize,
{
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut values_count = self.len();
		while values_count > 0 && self.values[values_count - 1] == self.default {
			values_count -= 1;
		}

		let mut seq = serializer.serialize_seq(Some(values_count + 1))?;
		seq.serialize_element(&Some(self.default.clone()))?;
		for e in self.values().take(values_count) {
			let some_e = Some(e);
			seq.serialize_element(if *e == self.default { &None } else { &some_e })?;
		}

		seq.end()
	}
}

#[repr(transparent)]
struct SecondaryMapVisitor<K, V> {
	marker: PhantomData<fn(K) -> V>,
}

impl<'de, K: EntityRef, V> Visitor<'de> for SecondaryMapVisitor<K, V>
where
	V: Clone + Deserialize<'de>,
{
	type Value = SecondaryMap<K, V>;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct SecondaryMap")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		match seq.next_element()? {
			Some(Some(default_value)) => {
				let default_value: V = default_value;
				let mut m = SecondaryMap::with_default(default_value.clone());
				let mut idx = 0;
				while let Some(val) = seq.next_element()? {
					let val: Option<_> = val;
					m[K::new(idx)] = val.unwrap_or_else(|| default_value.clone());
					idx += 1;
				}

				Ok(m)
			}
			_ => Err(DeError::custom("default value required")),
		}
	}
}
