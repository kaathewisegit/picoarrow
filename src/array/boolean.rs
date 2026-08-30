use bytemuck::cast_slice;

use std::fmt::{self, Debug};

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
		if V::IS_NULLABLE {
			self.len() - self.validity.count_ones()
		} else {
			0
		}
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
		self.validity.resize_bits(0);
		self.values.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Bool,
			children: Vec::new(),
			custom_metadata: Vec::new(),
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

	pub fn is_null(&self, index: usize) -> bool {
		!self.validity.get(index)
	}
}

impl ArrayBoolean<Nullable> {
	pub fn push(&mut self, value: Option<bool>) {
		match value {
			Some(v) => self.push_some(v),
			None => self.push_null(),
		}
	}

	pub fn push_some(&mut self, value: bool) {
		self.validity.resize_bits(self.len() + 1);
		self.validity.set_bit_on(self.len());
		self.values.resize_bits(self.len + 1);
		if value {
			self.values.set_bit_on(self.len);
		}
		self.len += 1;
	}

	pub fn get(&self, index: usize) -> Option<bool> {
		if self.validity.get(index) {
			Some(self.values.get(index))
		} else {
			None
		}
	}

	pub fn push_null(&mut self) {
		self.validity.resize_bits(self.len + 1);
		self.values.resize_bits(self.len + 1);
		self.len += 1;
	}
}

impl ArrayBoolean<NonNullable> {
	pub fn push(&mut self, value: bool) {
		self.values.resize_bits(self.len + 1);
		if value {
			self.values.set_bit_on(self.len);
		}
		self.len += 1;
	}

	pub fn get(&self, index: usize) -> bool {
		self.values.get(index)
	}
}

impl<V: Validity> Default for ArrayBoolean<V> {
	fn default() -> Self {
		Self::new()
	}
}

impl PartialEq for ArrayBoolean<NonNullable> {
	fn eq(&self, other: &Self) -> bool {
		self.len == other.len
			&& ValidityBuffer::compare(
				&self.values,
				&other.values,
				self.len(),
			)
	}
}
impl PartialEq for ArrayBoolean<Nullable> {
	fn eq(&self, other: &Self) -> bool {
		if self.len() != other.len() {
			return false;
		}
		// See `ArrayPrimitive` for implementation notes
		for i in 0..self.len() {
			if self.get(i) != other.get(i) {
				return false;
			}
		}
		true
	}
}
impl Eq for ArrayBoolean<NonNullable> {}
impl Eq for ArrayBoolean<Nullable> {}

impl Debug for ArrayBoolean<Nullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ArrayBoolean<Nullable>")
			.field("validity", &self.validity)
			.field("values", &self.values)
			.finish()
	}
}
impl Debug for ArrayBoolean<NonNullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ArrayBoolean<NonNullable>")
			.field("validity", &self.validity)
			.field("values", &self.values)
			.finish()
	}
}

impl<V: Validity> Clone for ArrayBoolean<V> {
	fn clone(&self) -> Self {
		Self {
			len: self.len,
			validity: self.validity.clone(),
			values: self.values.clone(),
		}
	}
}
