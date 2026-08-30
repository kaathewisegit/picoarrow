use bytemuck::cast_slice;

use std::fmt::{self, Debug};

use super::{Array, NonNullable, Nullable, Validity};
use crate::{
	Error,
	bitmap::ValidityBuffer,
	error::Result,
	schema::{DataType, Field},
	seal,
};

/// An array of binary slices
///
/// This is the small version which only supports up to 2GiB of data (combined
/// length of all items).
pub struct ArrayBinary<V: Validity> {
	validity: V::Container,
	offsets: Vec<i32>,
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
		if V::IS_NULLABLE {
			self.len() - self.validity.count_ones()
		} else {
			0
		}
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size()
			+ self.offsets.len() * size_of::<i32>()
			+ self.data.len()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.offsets.shrink_to_fit();
		self.data.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.validity.resize_bits(0);
		self.offsets.truncate(1);
		self.data.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Binary,
			children: Vec::new(),
			custom_metadata: Vec::new(),
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

	pub fn is_null(&self, index: usize) -> bool {
		!self.validity.get(index)
	}

	fn range(&self, index: usize) -> (usize, usize) {
		let start = self.offsets[index] as usize;
		let end = self.offsets[index + 1] as usize;
		(start, end)
	}

	fn get_unchecked(&self, index: usize) -> &[u8] {
		let (start, end) = self.range(index);
		&self.data[start..end]
	}

	fn get_mut_unchecked(&mut self, index: usize) -> &mut [u8] {
		let (start, end) = self.range(index);
		&mut self.data[start..end]
	}

	fn push_value(&mut self, bytes: &[u8]) -> Result<()> {
		let len = self.len();
		self.validity.resize_bits(len + 1);
		self.validity.set_bit_on(len);

		let start = *self.offsets.last().unwrap();
		let len = i32::try_from(bytes.len())
			.map_err(|_| Error::LengthOverflow)?;
		let end =
			start.checked_add(len).ok_or(Error::LengthOverflow)?;

		self.data.extend_from_slice(bytes);

		self.offsets.push(end);
		Ok(())
	}

	#[track_caller]
	fn check_index(&self, index: usize) {
		let len = self.len();
		if index >= self.len() {
			panic!("index {index} is out of bounds {len}");
		}
	}
}

impl ArrayBinary<Nullable> {
	pub fn get(&self, index: usize) -> Option<&[u8]> {
		self.check_index(index);
		if self.validity.get(index) {
			Some(self.get_unchecked(index))
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut [u8]> {
		self.check_index(index);
		if self.validity.get(index) {
			Some(self.get_mut_unchecked(index))
		} else {
			None
		}
	}

	pub fn push(&mut self, value: Option<&[u8]>) -> Result<()> {
		if let Some(value) = value {
			self.push_some(value)
		} else {
			self.push_null();
			Ok(())
		}
	}

	pub fn push_some(&mut self, value: &[u8]) -> Result<()> {
		self.push_value(value)
	}

	pub fn push_null(&mut self) {
		let len = self.len();
		self.validity.resize_bits(len + 1);

		let last = *self.offsets.last().unwrap();
		self.offsets.push(last);
	}
}

impl ArrayBinary<NonNullable> {
	pub fn get(&self, index: usize) -> &[u8] {
		self.check_index(index);
		self.get_unchecked(index)
	}

	pub fn get_mut(&mut self, index: usize) -> &mut [u8] {
		self.check_index(index);
		self.get_mut_unchecked(index)
	}

	pub fn push(&mut self, value: &[u8]) -> Result<()> {
		self.push_value(value)
	}
}

impl<V: Validity> Default for ArrayBinary<V> {
	fn default() -> Self {
		Self::new()
	}
}

impl Debug for ArrayBinary<Nullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("ArrayBinary<Nullable> { ... }")
	}
}
impl Debug for ArrayBinary<NonNullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("ArrayBinary<NonNullable> { ... }")
	}
}

impl PartialEq for ArrayBinary<NonNullable> {
	fn eq(&self, other: &Self) -> bool {
		self.offsets == other.offsets && self.data == other.data
	}
}
impl PartialEq for ArrayBinary<Nullable> {
	fn eq(&self, other: &Self) -> bool {
		if self.len() != other.len() {
			return false;
		}
		for i in 0..self.len() {
			if self.get(i) != other.get(i) {
				return false;
			}
		}
		true
	}
}
impl<V: Validity> Eq for ArrayBinary<V> where ArrayBinary<V>: PartialEq {}

impl<V: Validity> Clone for ArrayBinary<V> {
	fn clone(&self) -> Self {
		Self {
			validity: self.validity.clone(),
			offsets: self.offsets.clone(),
			data: self.data.clone(),
		}
	}
}

/// An array of strings
///
/// This is the small version which only supports up to 2GiB of strings
/// combined (the total length of all values).
pub struct ArrayUtf8<V: Validity> {
	validity: V::Container,
	offsets: Vec<i32>,
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
		if V::IS_NULLABLE {
			self.len() - self.validity.count_ones()
		} else {
			0
		}
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size()
			+ self.offsets.len() * size_of::<i32>()
			+ self.data.len()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.offsets.shrink_to_fit();
		self.data.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.validity.resize_bits(0);
		self.offsets.truncate(1);
		self.data.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Utf8,
			children: Vec::new(),
			custom_metadata: Vec::new(),
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

	pub fn is_null(&self, index: usize) -> bool {
		!self.validity.get(index)
	}

	fn range(&self, index: usize) -> (usize, usize) {
		let start = self.offsets[index] as usize;
		let end = self.offsets[index + 1] as usize;
		(start, end)
	}

	fn get_unchecked(&self, index: usize) -> &str {
		let (start, end) = self.range(index);
		&self.data[start..end]
	}

	fn get_mut_unchecked(&mut self, index: usize) -> &mut str {
		let (start, end) = self.range(index);
		&mut self.data[start..end]
	}

	fn push_value(&mut self, value: &str) -> Result<()> {
		let len = self.len();
		self.validity.resize_bits(len + 1);
		self.validity.set_bit_on(len);

		let start = *self.offsets.last().unwrap();
		let len = i32::try_from(value.len())
			.map_err(|_| Error::LengthOverflow)?;
		let end =
			start.checked_add(len).ok_or(Error::LengthOverflow)?;

		self.data.push_str(value);
		self.offsets.push(end);

		Ok(())
	}

	#[track_caller]
	fn check_index(&self, index: usize) {
		let len = self.len();
		if index >= self.len() {
			panic!("index {index} is out of bounds {len}");
		}
	}
}

impl ArrayUtf8<Nullable> {
	pub fn get(&self, index: usize) -> Option<&str> {
		self.check_index(index);
		if self.validity.get(index) {
			Some(self.get_unchecked(index))
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut str> {
		self.check_index(index);
		if self.validity.get(index) {
			Some(self.get_mut_unchecked(index))
		} else {
			None
		}
	}

	pub fn push(&mut self, value: Option<&str>) -> Result<()> {
		if let Some(value) = value {
			self.push_some(value)
		} else {
			self.push_null();
			Ok(())
		}
	}

	pub fn push_some(&mut self, value: &str) -> Result<()> {
		self.push_value(value)
	}

	pub fn push_null(&mut self) {
		let len = self.len();
		self.validity.resize_bits(len + 1);

		let last = *self.offsets.last().unwrap();
		self.offsets.push(last);
	}
}

impl ArrayUtf8<NonNullable> {
	pub fn get(&self, index: usize) -> &str {
		self.check_index(index);
		self.get_unchecked(index)
	}

	pub fn get_mut(&mut self, index: usize) -> &mut str {
		self.check_index(index);
		self.get_mut_unchecked(index)
	}

	pub fn push(&mut self, value: &str) -> Result<()> {
		self.push_value(value)
	}
}

impl<V: Validity> Default for ArrayUtf8<V> {
	fn default() -> Self {
		Self::new()
	}
}

impl Debug for ArrayUtf8<Nullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("ArrayUtf8<Nullable> { ... }")
	}
}
impl Debug for ArrayUtf8<NonNullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("ArrayUtf8<NonNullable> { ... }")
	}
}

impl PartialEq for ArrayUtf8<NonNullable> {
	fn eq(&self, other: &Self) -> bool {
		self.offsets == other.offsets && self.data == other.data
	}
}
impl PartialEq for ArrayUtf8<Nullable> {
	fn eq(&self, other: &Self) -> bool {
		if self.len() != other.len() {
			return false;
		}
		for i in 0..self.len() {
			println!("i = {i}");
			if self.get(i) != other.get(i) {
				return false;
			}
		}
		true
	}
}
impl<V: Validity> Eq for ArrayUtf8<V> where ArrayUtf8<V>: PartialEq {}

impl<V: Validity> Clone for ArrayUtf8<V> {
	fn clone(&self) -> Self {
		Self {
			validity: self.validity.clone(),
			offsets: self.offsets.clone(),
			data: self.data.clone(),
		}
	}
}
