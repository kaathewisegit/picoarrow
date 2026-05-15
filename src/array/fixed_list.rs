use flatbuffers::{FlatBufferBuilder, WIPOffset};

use super::{Array, Validity};
use crate::{
	bitmap::ValidityBuffer,
	fb::{Field, FieldArgs, FixedSizeList, FixedSizeListArgs, Type},
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

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>> {
		let name = builder.create_string(name);
		let type_union = FixedSizeList::create(
			builder,
			&FixedSizeListArgs {
				listSize: self.size,
			},
		)
		.as_union_value();

		let child_field = self.child.serialize_field(builder, "item");
		let children = builder.create_vector(&[child_field]);

		Field::create(
			builder,
			&FieldArgs {
				name: Some(name),
				nullable: V::IS_NULLABLE,
				type_type: Type::FixedSizeList,
				type_: Some(type_union),
				children: Some(children),
				dictionary: None,
				custom_metadata: None,
			},
		)
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

	pub fn push<F>(&mut self, f: F)
	where
		F: FnOnce(&mut A),
	{
		let before = self.child.len();
		f(&mut self.child);
		let after = self.child.len();
		assert_eq!(before - after, self.size as usize);

		self.validity.push(self.len, true);
		self.len += 1;
	}

	pub fn child(&self) -> &A {
		&self.child
	}
}
