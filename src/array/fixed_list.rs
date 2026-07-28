use super::{Array, Nullable, Validity};
use crate::{
	Error, Result,
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
	seal,
};

/// An array of lists with uniform length
pub struct ArrayFixedSizeList<A: Array, V: Validity> {
	len: usize,
	size: i32,
	validity: V::Container,
	child: A,
}

impl<A: Array, V: Validity> seal::Seal for ArrayFixedSizeList<A, V> {}
impl<A: Array, V: Validity> Array for ArrayFixedSizeList<A, V> {
	fn len(&self) -> usize {
		self.len
	}

	fn is_empty(&self) -> bool {
		self.len() == 0
	}

	fn null_count(&self) -> usize {
		if V::IS_NULLABLE {
			self.len() - self.validity.count_ones()
		} else {
			0
		}
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size() + self.child.memory_size()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.child.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.len = 0;
		self.validity.clear();
		self.child.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::FixedSizeList {
				list_size: self.size,
			},
			children: vec![self.child.make_field("item")],
			custom_metadata: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		self.child.walk_buffers(f);
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
		self.child.walk_nodes(f);
	}
}

impl<A: Array, V: Validity> ArrayFixedSizeList<A, V> {
	/// A new fixed-sized list array with items from `child`
	///
	/// # Panics
	///
	/// Panics if `size < 0`
	pub fn new(child: A, size: i32) -> Self {
		assert!(size > 0);
		Self {
			len: 0,
			size,
			validity: V::Container::new(),
			child,
		}
	}

	/// Push a new list to the array
	///
	/// This method takes a closure which returns the nested list.  After
	/// the closure yields `push` checks that the length has increased by
	/// exactly `size` and returns [`Error::WrongAppendLength`].
	///
	/// On failure the internal state of this array gets corrupted, so it
	/// can no longer be used.
	pub fn push<F>(&mut self, f: F) -> Result<()>
	where
		F: FnOnce(&mut A),
	{
		let before = self.child.len();
		f(&mut self.child);
		let after = self.child.len();

		if after - before != self.size as usize {
			return Err(Error::WrongAppendLength {
				expected: self.size as usize,
				got: after - before,
			});
		}

		self.validity.resize_bits(self.len + 1);
		self.validity.set_bit(self.len, true);
		self.len += 1;

		Ok(())
	}

	pub fn child(&self) -> &A {
		&self.child
	}

	pub fn is_null(&self, index: usize) -> bool {
		self.validity.is_null(index)
	}
}

impl<A: Array> ArrayFixedSizeList<A, Nullable> {
	pub fn push_null<F>(&mut self, f: F) -> Result<()>
	where
		F: FnOnce(&mut A),
	{
		let before = self.child.len();
		f(&mut self.child);
		let after = self.child.len();

		if after - before != self.size as usize {
			return Err(Error::WrongAppendLength {
				expected: self.size as usize,
				got: after - before,
			});
		}

		self.validity.resize_bits(self.len + 1);
		self.validity.set_bit(self.len, false);
		self.len += 1;

		Ok(())
	}
}
