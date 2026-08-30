use bytemuck::{AnyBitPattern, NoUninit, cast_slice};

use std::{
	any::type_name,
	borrow::Borrow,
	fmt::{self, Debug},
	ops::{Deref, DerefMut},
};

use super::{Array, NonNullable, Nullable, Validity};
use crate::{
	bitmap::ValidityBuffer,
	fb::Precision,
	schema::{DataType, Field},
	seal,
};

pub trait Primitive:
	NoUninit + AnyBitPattern + Default + Debug + PartialEq + seal::Seal
{
	fn data_type() -> DataType;

	type Bytes: Borrow<[u8]> + PartialEq + Debug;
	fn to_le_bytes(self) -> Self::Bytes;
}

macro_rules! impl_primitive_int {
	($type:ty, $byte_size:expr, $is_signed:expr) => {
		impl seal::Seal for $type {}
		impl Primitive for $type {
			fn data_type() -> DataType {
				DataType::Int {
					bit_width: $byte_size * 8,
					is_signed: $is_signed,
				}
			}

			type Bytes = [u8; $byte_size];
			fn to_le_bytes(self) -> [u8; $byte_size] {
				self.to_le_bytes()
			}
		}
	};
}
impl_primitive_int!(u8, 1, false);
impl_primitive_int!(u16, 2, false);
impl_primitive_int!(u32, 4, false);
impl_primitive_int!(u64, 8, false);
impl_primitive_int!(i8, 1, true);
impl_primitive_int!(i16, 2, true);
impl_primitive_int!(i32, 4, true);
impl_primitive_int!(i64, 8, true);

macro_rules! impl_primitive_float {
	($type:ty, $byte_size:expr, $kind:ident) => {
		impl seal::Seal for $type {}
		impl Primitive for $type {
			fn data_type() -> DataType {
				DataType::FloatingPoint {
					precision: Precision::$kind,
				}
			}

			type Bytes = [u8; $byte_size];
			fn to_le_bytes(self) -> [u8; $byte_size] {
				self.to_le_bytes()
			}
		}
	};
}
impl_primitive_float!(f32, 4, SINGLE);
impl_primitive_float!(f64, 8, DOUBLE);
#[cfg(feature = "half")]
impl_primitive_float!(half::f16, 2, HALF);

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
		if V::IS_NULLABLE {
			self.len() - self.validity.count_ones()
		} else {
			0
		}
	}

	fn memory_size(&self) -> usize {
		self.validity.memory_size() + self.len() * size_of::<T>()
	}

	fn shrink_to_fit(&mut self) {
		self.validity.shrink_to_fit();
		self.values.shrink_to_fit();
	}

	fn clear(&mut self) {
		self.validity.resize_bits(0);
		self.values.clear();
	}

	fn make_field(&self, name: &str) -> Field {
		Field {
			name: name.to_owned(),
			nullable: V::IS_NULLABLE,
			type_: T::data_type(),
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

impl<T: Primitive, V: Validity> ArrayPrimitive<T, V> {
	/// Constructs a new, empty array
	pub fn new() -> Self {
		Self {
			validity: V::Container::new(),
			values: Vec::new(),
		}
	}

	/// Constructs a new array with `values`
	///
	/// Note that it takes `values` by value, reusing the allocation.
	pub fn from_vec(values: Vec<T>) -> Self {
		let mut validity = V::Container::new();
		let len = values.len();
		validity.resize_bits(values.len());
		validity.set_bits_on(0, len);
		Self { validity, values }
	}

	/// Returns `true` if the item at `index` is null
	pub fn is_null(&self, index: usize) -> bool {
		!self.validity.get(index)
	}

	/// Copies and appends `values` to the end of the array
	pub fn extend_from_slice(&mut self, values: &[T]) {
		let old_len = self.len();
		let values_len = values.len();
		self.values.extend_from_slice(values);
		self.validity.resize_bits(old_len + values_len);
		self.validity.set_bits_on(old_len, values_len);
	}
}

impl<T: Primitive> ArrayPrimitive<T, Nullable> {
	pub fn push(&mut self, value: Option<T>) {
		match value {
			Some(v) => self.push_some(v),
			None => self.push_null(),
		}
	}

	pub fn push_some(&mut self, value: T) {
		self.validity.resize_bits(self.len() + 1);
		self.validity.set_bit_on(self.len());
		self.values.push(value);
	}

	pub fn push_null(&mut self) {
		let len = self.len();
		self.validity.resize_bits(len + 1);
		self.values.push(T::default());
	}

	pub fn get(&self, index: usize) -> Option<&T> {
		if self.validity.get(index) {
			Some(&self.values[index])
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
		if self.validity.get(index) {
			Some(&mut self.values[index])
		} else {
			None
		}
	}
}

impl<T: Primitive> ArrayPrimitive<T, NonNullable> {
	pub fn push(&mut self, value: T) {
		self.values.push(value);
	}

	pub fn get(&self, index: usize) -> &T {
		&self.values[index]
	}

	pub fn get_mut(&mut self, index: usize) -> &mut T {
		&mut self.values[index]
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

impl<T: Primitive> From<Vec<T>> for ArrayPrimitive<T, NonNullable> {
	fn from(values: Vec<T>) -> Self {
		Self::from_vec(values)
	}
}

impl<T: Primitive> Debug for ArrayPrimitive<T, NonNullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let name = format!(
			"ArrayPrimitive<{}, NonNullable>",
			type_name::<T>()
		);
		f.debug_struct(&name).field("values", &self.values).finish()
	}
}
impl<T: Primitive> Debug for ArrayPrimitive<T, Nullable> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let name = format!(
			"ArrayPrimitive<{}, Nullable>",
			type_name::<T>()
		);
		f.debug_struct(&name)
			.field("validity", &self.validity)
			.field("values", &self.values)
			.finish()
	}
}

impl<T: Primitive> Extend<Option<T>> for ArrayPrimitive<T, Nullable> {
	fn extend<I: IntoIterator<Item = Option<T>>>(&mut self, iter: I) {
		let iter = iter.into_iter();
		let (lower, _) = iter.size_hint();
		self.values.reserve(lower);
		self.validity.reserve(lower.div_ceil(8));
		for item in iter {
			self.push(item);
		}
	}
}

impl<T: Primitive> Extend<T> for ArrayPrimitive<T, NonNullable> {
	fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
		let iter = iter.into_iter();
		let (lower, _) = iter.size_hint();
		self.values.reserve(lower);
		for item in iter {
			self.push(item);
		}
	}
}

impl<T: Primitive> PartialEq for ArrayPrimitive<T, NonNullable> {
	fn eq(&self, other: &Self) -> bool {
		self.values == other.values
	}
}

impl<T: Primitive> PartialEq for ArrayPrimitive<T, Nullable> {
	fn eq(&self, other: &Self) -> bool {
		// This is a pretty poor implementation.  It's serial and does
		// redundant range checks.  A smarter implementation would do
		// simd checks on both values and nullability mask bytes, but
		// that would require a lot of abstractions and effort for a
		// pretty rare operation.
		for i in 0..self.len() {
			if self.get(i) != other.get(i) {
				return false;
			}
		}
		true
	}
}

impl<T: Primitive, V: Validity> Clone for ArrayPrimitive<T, V> {
	fn clone(&self) -> Self {
		Self {
			validity: self.validity.clone(),
			values: self.values.clone(),
		}
	}
}

/// An array of [`u8`]'s.
///
/// It is restricted by the API of [`ArrayPrimitive`] and treats each byte as an
/// individual item.  Use [`ArrayBinary`][b] for keeping track of
/// variable-length slices or [`ArrayFixedBinary`][fb] for arbitrary values of
/// the same size[^uuid].
///
/// [^uuid]: That's how Arrow [defines UUIDs][uuid], for example.
///
/// [b]: super::ArrayBinary
/// [fb]: super::ArrayFixedBinary
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
/// An array of [`f16`][half::f16]'s from the `half` crate
#[cfg(feature = "half")]
pub type ArrayF16<V> = ArrayPrimitive<half::f16, V>;
