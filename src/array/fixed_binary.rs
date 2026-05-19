use super::{Array, NonNullable, Nullable, Validity};
use crate::{
	Error, Result,
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
	seal,
};

/// An array of fixed-size binary values
pub struct ArrayFixedBinary<V: Validity> {
	validity: V::Container,
	data: Vec<u8>,
	byte_width: i32,
}

impl<V: Validity> seal::Seal for ArrayFixedBinary<V> {}
impl<V: Validity> Array for ArrayFixedBinary<V> {
	fn len(&self) -> usize {
		self.data.len() / self.byte_width as usize
	}

	fn is_empty(&self) -> bool {
		self.data.is_empty()
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size() + self.data.len()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.data.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.validity.clear();
		self.data.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::FixedSizeBinary {
				byte_width: self.byte_width,
			},
			children: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		f(&self.data);
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
	}
}

impl<V: Validity> ArrayFixedBinary<V> {
	fn byte_width(&self) -> usize {
		self.byte_width as usize
	}

	/// Returns an array where each element is `byte_width` bytes
	pub fn new(byte_width: i32) -> Self {
		assert!(byte_width > 0);
		Self {
			validity: V::Container::new(),
			data: Vec::new(),
			byte_width,
		}
	}

	/// Append a fixed-size binary value
	///
	/// Returns [`Error::WrongBinaryAppendLength`] if the length of `bytes`
	/// does not match the element size of the array.
	pub fn push(&mut self, bytes: &[u8]) -> Result<()> {
		if bytes.len() != self.byte_width() {
			return Err(Error::WrongBinaryAppendLength {
				expected: self.byte_width,
				got: bytes.len(),
			});
		}
		self.validity.push(self.len(), true);
		self.data.extend_from_slice(bytes);
		Ok(())
	}

	pub fn is_null(&self, index: usize) -> bool {
		self.validity.is_null(index)
	}

	fn get_unchecked(&self, index: usize) -> &[u8] {
		let start = index * self.byte_width();
		&self.data[start..start + self.byte_width()]
	}

	fn get_mut_unchecked(&mut self, index: usize) -> &mut [u8] {
		let start = index * self.byte_width();
		let end = start + self.byte_width();
		&mut self.data[start..end]
	}
}

impl ArrayFixedBinary<Nullable> {
	pub fn get(&self, index: usize) -> Option<&[u8]> {
		if self.is_null(index) {
			None
		} else {
			Some(self.get_unchecked(index))
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut [u8]> {
		if self.is_null(index) {
			None
		} else {
			Some(self.get_mut_unchecked(index))
		}
	}

	pub fn push_null(&mut self) {
		let len = self.len();
		ValidityBuffer::push(&mut self.validity, len, false);
		self.data.resize(self.data.len() + self.byte_width(), 0);
	}
}

impl ArrayFixedBinary<NonNullable> {
	pub fn get(&self, index: usize) -> &[u8] {
		self.get_unchecked(index)
	}

	pub fn get_mut(&mut self, index: usize) -> &mut [u8] {
		self.get_mut_unchecked(index)
	}
}
