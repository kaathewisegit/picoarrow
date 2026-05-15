use super::{Array, Validity};
use crate::{
	Error, Result,
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
};

pub struct ArrayFixedSizeList<A: Array, V: Validity> {
	len: usize,
	size: i32,
	validity: V::Container,
	child: A,
}

impl<A: Array, V: Validity> Array for ArrayFixedSizeList<A, V> {
	fn len(&self) -> usize {
		self.len
	}

	fn is_empty(&self) -> bool {
		self.len() == 0
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::FixedSizeList {
				list_size: self.size,
			},
			children: vec![self.child.make_field("item")],
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
	pub fn new(child: A, size: i32) -> Self {
		Self {
			len: 0,
			size,
			validity: V::Container::new(),
			child,
		}
	}

	pub fn push<F>(&mut self, f: F) -> Result<()>
	where
		F: FnOnce(&mut A),
	{
		let before = self.child.len();
		f(&mut self.child);
		let after = self.child.len();

		if after - before != self.size as usize {
			return Err(Error::WrongNestedLength {
				expected: self.size,
				got: after - before,
			});
		}

		self.validity.push(self.len, true);
		self.len += 1;

		Ok(())
	}

	pub fn child(&self) -> &A {
		&self.child
	}
}
