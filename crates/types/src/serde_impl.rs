use core::{
	fmt::{Formatter, Result as FmtResult},
	marker::PhantomData,
};

use serde::{
	de::{Deserialize, Deserializer, Error as DeError, SeqAccess, Visitor},
	ser::{Serialize, Serializer},
};

use super::{Register, RegisterType};

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
