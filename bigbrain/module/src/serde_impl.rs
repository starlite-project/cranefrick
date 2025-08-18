use std::fmt::{Formatter, Result as FmtResult};

use cranelift_codegen::entity::PrimaryMap;
use hashbrown::HashMap;
use serde::{
	Deserialize, Deserializer, Serialize, Serializer,
	de::{Error as DeError, IgnoredAny, MapAccess, SeqAccess, Unexpected, Visitor},
	ser::SerializeStruct,
};

use super::{
	DataDeclaration, DataId, FuncId, FuncOrDataId, FunctionDeclaration, ModuleDeclarations,
};

impl<'de> Deserialize<'de> for ModuleDeclarations {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_struct(
			"ModuleDeclarations",
			&["_version_marker", "functions", "data_objects"],
			ModuleDeclarationsVisitor,
		)
	}
}

impl Serialize for ModuleDeclarations {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let Self {
			functions,
			data_objects,
			_version_marker,
			..
		} = self;

		let mut state = serializer.serialize_struct("ModuleDeclarations", 4)?;
		state.serialize_field("_version_marker", _version_marker)?;
		state.serialize_field("functions", &functions)?;
		state.serialize_field("data_objects", &data_objects)?;
		state.end()
	}
}

struct ModuleDeclarationsVisitor;

impl<'de> Visitor<'de> for ModuleDeclarationsVisitor {
	type Value = ModuleDeclarations;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("struct ModuleDeclarations")
	}

	fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let Some(_version_marker) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				0,
				&"struct ModuleDeclarations with 4 elements",
			));
		};

		let Some(functions) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				1,
				&"struct ModuleDeclarations with 4 elements",
			));
		};

		let Some(data_objects) = seq.next_element()? else {
			return Err(DeError::invalid_length(
				2,
				&"struct ModuleDeclarations with 4 elements",
			));
		};

		let names = get_names(&functions, &data_objects)?;
		Ok(ModuleDeclarations {
			_version_marker,
			names,
			functions,
			data_objects,
		})
	}

	fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut _version_marker = None;
		let mut functions = None;
		let mut data_objects = None;

		while let Some(key) = map.next_key()? {
			match key {
				ModuleDeclarationsField::VersionMarker => {
					if _version_marker.is_some() {
						return Err(DeError::duplicate_field("_version_marker"));
					}

					_version_marker = Some(map.next_value()?);
				}
				ModuleDeclarationsField::Functions => {
					if functions.is_some() {
						return Err(DeError::duplicate_field("functions"));
					}

					functions = Some(map.next_value()?);
				}
				ModuleDeclarationsField::DataObjects => {
					if data_objects.is_some() {
						return Err(DeError::duplicate_field("data_objects"));
					}

					data_objects = Some(map.next_value()?);
				}
				ModuleDeclarationsField::Ignore => {
					_ = map.next_value::<IgnoredAny>()?;
				}
			}
		}

		let Some(_version_marker) = _version_marker else {
			return Err(DeError::missing_field("_version_marker"));
		};

		let Some(functions) = functions else {
			return Err(DeError::missing_field("functions"));
		};

		let Some(data_objects) = data_objects else {
			return Err(DeError::missing_field("data_objects"));
		};

		let names = get_names(&functions, &data_objects)?;

		Ok(ModuleDeclarations {
			_version_marker,
			names,
			functions,
			data_objects,
		})
	}
}

struct ModuleDeclarationsFieldVisitor;

impl Visitor<'_> for ModuleDeclarationsFieldVisitor {
	type Value = ModuleDeclarationsField;

	fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
		formatter.write_str("field identifier")
	}

	fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
		Ok(match v {
			0 => ModuleDeclarationsField::VersionMarker,
			1 => ModuleDeclarationsField::Functions,
			2 => ModuleDeclarationsField::DataObjects,
			_ => ModuleDeclarationsField::Ignore,
		})
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
		Ok(match v {
			"_version_marker" => ModuleDeclarationsField::VersionMarker,
			"functions" => ModuleDeclarationsField::Functions,
			"data_objects" => ModuleDeclarationsField::DataObjects,
			_ => ModuleDeclarationsField::Ignore,
		})
	}

	fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E> {
		Ok(match v {
			b"_version_marker" => ModuleDeclarationsField::VersionMarker,
			b"functions" => ModuleDeclarationsField::Functions,
			b"data_objects" => ModuleDeclarationsField::DataObjects,
			_ => ModuleDeclarationsField::Ignore,
		})
	}
}

enum ModuleDeclarationsField {
	VersionMarker,
	Functions,
	DataObjects,
	Ignore,
}

impl<'de> Deserialize<'de> for ModuleDeclarationsField {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_identifier(ModuleDeclarationsFieldVisitor)
	}
}

fn get_names<E: DeError>(
	functions: &PrimaryMap<FuncId, FunctionDeclaration>,
	data_objects: &PrimaryMap<DataId, DataDeclaration>,
) -> Result<HashMap<String, FuncOrDataId>, E> {
	let mut names = HashMap::new();

	for (func_id, decl) in functions {
		if let Some(name) = &decl.name {
			let old = names.insert(name.clone(), FuncOrDataId::Func(func_id));
			if old.is_some() {
				return Err(E::invalid_value(
					Unexpected::Other("duplicate name"),
					&"FunctionDeclaration's with no duplicate names",
				));
			}
		}
	}

	for (data_id, decl) in data_objects {
		if let Some(name) = &decl.name {
			let old = names.insert(name.clone(), FuncOrDataId::Data(data_id));
			if old.is_some() {
				return Err(E::invalid_value(
					Unexpected::Other("duplicate name"),
					&"DataDeclaration's with no duplicate names",
				));
			}
		}
	}

	Ok(names)
}
