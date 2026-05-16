use bytemuck::{NoUninit, cast_slice};

use std::ops::{Deref, DerefMut};

use super::{Array, NonNullable, Validity};
use crate::{
	bitmap::ValidityBuffer,
	fb::Precision,
	schema::{DataType, Field},
	seal,
};

pub trait Primitive: NoUninit + seal::Seal {
	fn data_type() -> DataType;
}

macro_rules! impl_primitive_int {
	($type:ty, $bit_width:expr, $is_signed:expr) => {
		impl seal::Seal for $type {}
		impl Primitive for $type {
			fn data_type() -> DataType {
				DataType::Int {
					bit_width: $bit_width,
					is_signed: $is_signed,
				}
			}
		}
	};
}
impl_primitive_int!(u8, 8, false);
impl_primitive_int!(u16, 16, false);
impl_primitive_int!(u32, 32, false);
impl_primitive_int!(u64, 64, false);
impl_primitive_int!(i8, 8, true);
impl_primitive_int!(i16, 16, true);
impl_primitive_int!(i32, 32, true);
impl_primitive_int!(i64, 64, true);

macro_rules! impl_primitive_float {
	($type:ty, $kind:ident) => {
		impl seal::Seal for $type {}
		impl Primitive for $type {
			fn data_type() -> DataType {
				DataType::FloatingPoint {
					precision: Precision::$kind,
				}
			}
		}
	};
}
// TODO: feature-gated half
impl_primitive_float!(f32, SINGLE);
impl_primitive_float!(f64, DOUBLE);

/// An array of primitive (uniform size, copyable) arrow types
pub struct ArrayPrimitive<T: Primitive, V: Validity> {
	validity: V::Container,
	values: Vec<T>,
}

impl<T: Primitive, V: Validity> seal::Seal for ArrayPrimitive<T, V> {}
impl<T: Primitive, V: Validity> Array for ArrayPrimitive<T, V> {
	fn len(&self) -> usize {
		self.values.len()
	}

	fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	fn null_count(&self) -> usize {
		self.validity.null_count()
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size() + self.len() * size_of::<T>()
	}

	fn clear(&mut self) {
		self.validity.clear();
		self.values.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: T::data_type(),
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

impl<T: Primitive, V: Validity> ArrayPrimitive<T, V> {
	pub fn new() -> Self {
		Self {
			validity: V::Container::new(),
			values: Vec::new(),
		}
	}

	pub fn push(&mut self, value: T) {
		self.validity.push(self.len(), true);
		self.values.push(value);
	}

	pub fn is_null(&self, index: usize) -> bool {
		self.validity.is_null(index)
	}

	pub fn get(&self, index: usize) -> Option<&T> {
		if self.is_null(index) {
			None
		} else {
			Some(&self.values[index])
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
		if self.is_null(index) {
			None
		} else {
			Some(&mut self.values[index])
		}
	}
}

impl<T: Primitive, V: Validity> Default for ArrayPrimitive<T, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Primitive> Deref for ArrayPrimitive<T, NonNullable> {
	type Target = [T];

	fn deref(&self) -> &[T] {
		&self.values
	}
}

impl<T: Primitive> DerefMut for ArrayPrimitive<T, NonNullable> {
	fn deref_mut(&mut self) -> &mut [T] {
		&mut self.values
	}
}

/// An array of [`u8`]'s.
///
/// It is restricted by the API of [`ArrayPrimitive`] and treats each byte as an
/// individual item.  Use [`ArrayBinary`][b] for keeping track of
/// variable-length slices or `FixedSizeBinary` (TODO) for arbitrary values of
/// the same size[^uuid].
///
/// [^uuid]: That's how Arrow [defines UUIDs][uuid], for example.
///
/// [b]: super::ArrayBinary
/// [uuid]: https://arrow.apache.org/docs/format/Columnar.html#:~:text=Arrow%20specifies,messages
pub type ArrayU8<V> = ArrayPrimitive<u8, V>;
/// An array of [`u16`]'s
pub type ArrayU16<V> = ArrayPrimitive<u16, V>;
/// An array of [`u32`]'s
pub type ArrayU32<V> = ArrayPrimitive<u32, V>;
/// An array of [`u64`]'s
pub type ArrayU64<V> = ArrayPrimitive<u64, V>;

/// An array of [`i8`]'s
pub type ArrayI8<V> = ArrayPrimitive<i8, V>;
/// An array of [`i16`]'s
pub type ArrayI16<V> = ArrayPrimitive<i16, V>;
/// An array of [`i32`]'s
pub type ArrayI32<V> = ArrayPrimitive<i32, V>;
/// An array of [`i64`]'s
pub type ArrayI64<V> = ArrayPrimitive<i64, V>;

/// An array of [`f32`]'s, called single precision floats in Arrow
pub type ArrayF32<V> = ArrayPrimitive<f32, V>;
/// An array of [`f64`]'s, called double precision floats in Arrow
pub type ArrayF64<V> = ArrayPrimitive<f64, V>;
