use bytemuck::cast_slice;

use super::{Array, Validity};
use crate::{
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
	seal,
};

/// An array of binary slices
///
/// This is the small version which only supports up to 2GiB of data (combined
/// length of all items).
pub struct ArrayBinary<V: Validity> {
	validity: V::Container,
	offsets: Vec<u32>,
	data: Vec<u8>,
}

impl<V: Validity> seal::Seal for ArrayBinary<V> {}
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

	fn memory_size(&self) -> usize {
		self.validity.memory_size()
			+ self.offsets.len() * size_of::<u32>()
			+ self.data.len()
	}

	fn clear(&mut self) {
		self.validity.clear();
		self.offsets.resize(1, 0);
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Binary,
			children: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		f(cast_slice(&self.offsets));
		f(&self.data);
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
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

/// An array of string slices
///
/// This is the small version which only supports up to 2GiB of data (combined
/// length of all items).
pub struct ArrayUtf8<V: Validity> {
	validity: V::Container,
	offsets: Vec<u32>,
	data: String,
}

impl<V: Validity> seal::Seal for ArrayUtf8<V> {}
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

	fn memory_size(&self) -> usize {
		self.validity.memory_size()
			+ self.offsets.len() * size_of::<u32>()
			+ self.data.len()
	}

	fn clear(&mut self) {
		self.validity.clear();
		self.offsets.resize(1, 0);
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Utf8,
			children: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		f(cast_slice(&self.offsets));
		f(self.data.as_bytes());
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
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
