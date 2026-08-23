use super::{Array, Nullable, Validity};
use crate::{
	Error, Result,
	bitmap::ValidityBuffer,
	schema::{DataType, Field},
	seal,
};

pub trait StructFields: seal::Seal {
	fn for_each(&self, f: &mut dyn FnMut(&dyn Array));
	fn for_each_mut(&mut self, f: &mut dyn FnMut(&mut dyn Array));
}

pub struct ArrayStruct<F: StructFields, V: Validity> {
	len: usize,
	names: Vec<String>,
	fields: F,
	validity: V::Container,
}

impl<F: StructFields, V: Validity> seal::Seal for ArrayStruct<F, V> {}
impl<F: StructFields, V: Validity> Array for ArrayStruct<F, V> {
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
		let mut size = self.validity.memory_size();
		self.fields.for_each(&mut |arr| {
			size += arr.memory_size();
		});
		size
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.fields.for_each_mut(&mut |arr| {
			arr.shrink_to_fit();
		});
	}

	fn clear(&mut self) {
		self.len = 0;
		self.validity.resize_bits(0);
		self.fields.for_each_mut(&mut |arr| {
			arr.clear();
		});
	}

	fn make_field(&self, name: &str) -> Field {
		let mut children = Vec::with_capacity(self.names.len());
		self.fields.for_each(&mut |arr| {
			let i = children.len();
			children.push(arr.make_field(&self.names[i]));
		});
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: DataType::Struct,
			children,
			custom_metadata: Vec::new(),
		}
	}

	fn walk_buffers(&self, f: &mut dyn FnMut(&[u8])) {
		f(self.validity.buffer());
		self.fields.for_each(&mut |arr| {
			arr.walk_buffers(f);
		});
	}

	fn walk_nodes(&self, f: &mut dyn FnMut(usize, usize)) {
		f(self.len(), self.null_count());
		self.fields.for_each(&mut |arr| {
			arr.walk_nodes(f);
		});
	}
}

impl<F: StructFields, V: Validity> ArrayStruct<F, V> {
	pub fn new(names: Vec<String>, fields: F) -> Self {
		Self {
			len: 0,
			names,
			fields,
			validity: V::Container::new(),
		}
	}

	pub fn push<Func>(&mut self, f: Func) -> Result<()>
	where
		Func: FnOnce(&mut F),
	{
		f(&mut self.fields);

		let expected = self.len + 1;
		let mut ok = true;
		self.fields.for_each(&mut |arr| {
			if arr.len() != expected {
				ok = false;
			}
		});

		if !ok {
			return Err(Error::WrongAppendLength {
				expected,
				got: self.len,
			});
		}

		self.validity.resize_bits(self.len + 1);
		self.validity.set_bit_on(self.len);
		self.len += 1;

		Ok(())
	}

	pub fn fields(&self) -> &F {
		&self.fields
	}

	pub fn is_null(&self, index: usize) -> bool {
		!self.validity.get(index)
	}
}

impl<F: StructFields> ArrayStruct<F, Nullable> {
	pub fn push_null<Func>(&mut self, f: Func) -> Result<()>
	where
		Func: FnOnce(&mut F),
	{
		f(&mut self.fields);

		let expected = self.len + 1;
		let mut ok = true;
		self.fields.for_each(&mut |arr| {
			if arr.len() != expected {
				ok = false;
			}
		});

		if !ok {
			return Err(Error::WrongAppendLength {
				expected,
				got: self.len,
			});
		}

		self.validity.resize_bits(self.len + 1);
		self.validity.set_bit_off(self.len);
		self.len += 1;

		Ok(())
	}
}

macro_rules! impl_struct_fields {
    ($($T:ident : $idx:tt),* $(,)?) => {
		impl<$($T: Array,)*> seal::Seal for ($($T,)*) {}

		impl<$($T: Array,)*> StructFields for ($($T,)*) {
			fn for_each(&self, f: &mut dyn FnMut(&dyn Array)) {
				$(f(&self.$idx);)*
			}

			fn for_each_mut(&mut self, f: &mut dyn FnMut(&mut dyn Array)) {
				$(f(&mut self.$idx);)*
			}
		}
    };
}

impl_struct_fields!(A: 0);
impl_struct_fields!(A: 0, B: 1);
impl_struct_fields!(A: 0, B: 1, C: 2);
impl_struct_fields!(A: 0, B: 1, C: 2, D: 3);
impl_struct_fields!(A: 0, B: 1, C: 2, D: 3, E: 4);
impl_struct_fields!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5);
impl_struct_fields!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6);
impl_struct_fields!(A: 0, B: 1, C: 2, D: 3, E: 4, F: 5, G: 6, H: 7);
