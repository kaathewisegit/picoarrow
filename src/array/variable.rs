use flatbuffers::{FlatBufferBuilder, WIPOffset};

use super::{Array, Validity};
use crate::{
	bitmap::ValidityBuffer,
	fb::{Binary, BinaryArgs, Field, FieldArgs, Type, Utf8, Utf8Args},
};

pub struct ArrayBinary<V: Validity> {
	validity: V::Container,
	offsets: Vec<u32>,
	data: Vec<u8>,
}

impl<V: Validity> Array for ArrayBinary<V> {
	fn len(&self) -> usize {
		self.offsets.len() - 1
	}

	fn is_empty(&self) -> bool {
		self.offsets.len() == 1
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>> {
		let name = builder.create_string(name);
		let type_union = Binary::create(builder, &BinaryArgs {})
			.as_union_value();

		Field::create(
			builder,
			&FieldArgs {
				name: Some(name),
				nullable: V::IS_NULLABLE,
				type_type: Type::Binary,
				type_: Some(type_union),
				dictionary: None,
				children: None,
				custom_metadata: None,
			},
		)
	}

	fn walk_buffers<F>(&self, mut f: F)
	where
		F: FnMut(&[u8]),
	{
		f(self.validity.buffer());
		f(&self.data);
	}
}

impl<V: Validity> ArrayBinary<V> {
	pub fn new() -> Self {
		Self {
			validity: V::Container::new(),
			offsets: vec![0],
			data: Vec::new(),
		}
	}

	pub fn push(&mut self, bytes: &[u8]) {
		let start = *self.offsets.last().unwrap();
		// TODO: check length
		let end = start + bytes.len() as u32;

		self.data.extend_from_slice(bytes);

		self.offsets.push(end);
	}
}

impl<V: Validity> Default for ArrayBinary<V> {
	fn default() -> Self {
		Self::new()
	}
}

pub struct ArrayUtf8<V: Validity> {
	validity: V::Container,
	offsets: Vec<u32>,
	data: String,
}

impl<V: Validity> Array for ArrayUtf8<V> {
	fn len(&self) -> usize {
		self.offsets.len() - 1
	}

	fn is_empty(&self) -> bool {
		self.offsets.len() == 1
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>> {
		let name = builder.create_string(name);
		let type_union =
			Utf8::create(builder, &Utf8Args {}).as_union_value();

		Field::create(
			builder,
			&FieldArgs {
				name: Some(name),
				nullable: V::IS_NULLABLE,
				type_type: Type::Utf8,
				type_: Some(type_union),
				dictionary: None,
				children: None,
				custom_metadata: None,
			},
		)
	}

	fn walk_buffers<F>(&self, mut f: F)
	where
		F: FnMut(&[u8]),
	{
		f(self.validity.buffer());
		f(self.data.as_bytes());
	}
}

impl<V: Validity> ArrayUtf8<V> {
	pub fn new() -> Self {
		Self {
			validity: V::Container::new(),
			offsets: vec![0],
			data: String::new(),
		}
	}

	pub fn push(&mut self, value: &str) {
		let start = *self.offsets.last().unwrap();
		// TODO: check length
		let end = start + value.len() as u32;

		self.data.push_str(value);

		self.offsets.push(end);
	}
}

impl<V: Validity> Default for ArrayUtf8<V> {
	fn default() -> Self {
		Self::new()
	}
}
