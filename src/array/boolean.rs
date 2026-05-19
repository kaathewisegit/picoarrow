use bytemuck::cast_slice;

use super::{Array, NonNullable, Validity};
use crate::{
	array::Nullable,
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
	seal,
};

/// An array of booleans
///
/// Unlike other [primitive arrays][`super::ArrayPrimitive`], this array
/// compresses its items storing one boolean per bit (similar to the optional
/// `std::vector<bool>` optimization).
pub struct ArrayBoolean<V: Validity> {
	len: usize,
	validity: V::Container,
	values: Vec<u8>,
}

impl<V: Validity> seal::Seal for ArrayBoolean<V> {}
impl<V: Validity> Array for ArrayBoolean<V> {
	fn len(&self) -> usize {
		self.len
	}

	fn is_empty(&self) -> bool {
		self.len == 0
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size() + self.values.len()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.values.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.len = 0;
		self.validity.clear();
		self.values.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Bool,
			children: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		let value_buf: &[u8] = cast_slice(&self.values);
		f(value_buf);
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
	}
}

impl<V: Validity> ArrayBoolean<V> {
	pub fn new() -> Self {
		Self {
			len: 0,
			validity: V::Container::new(),
			values: Vec::new(),
		}
	}

	pub fn push(&mut self, value: bool) {
		self.validity.push(self.len(), true);
		ValidityBuffer::push(&mut self.values, self.len, value);
		self.len += 1;
	}

	pub fn is_null(&self, index: usize) -> bool {
		self.validity.is_null(index)
	}
}

impl ArrayBoolean<Nullable> {
	pub fn get(&self, index: usize) -> Option<bool> {
		if self.is_null(index) {
			None
		} else {
			Some(!self.values.is_null(index))
		}
	}

	pub fn push_null(&mut self) {
		ValidityBuffer::push(&mut self.validity, self.len, false);
		ValidityBuffer::push(&mut self.values, self.len, false);
		self.len += 1;
	}
}

impl ArrayBoolean<NonNullable> {
	pub fn get(&self, index: usize) -> bool {
		!self.values.is_null(index)
	}
}

impl<V: Validity> Default for ArrayBoolean<V> {
	fn default() -> Self {
		Self::new()
	}
}
