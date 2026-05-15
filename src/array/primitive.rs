use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use std::ops::{Deref, DerefMut};

use super::{Array, NonNullable, Validity};
use crate::{
	bitmap::ValidityBuffer,
	fb::{
		Field, FieldArgs, FloatingPoint, FloatingPointArgs, Int,
		IntArgs, Precision, Type,
	},
};

pub trait Primitive {
	fn type_discriminant() -> Type;

	fn type_union<'fbb>(
		builder: &mut FlatBufferBuilder<'fbb>,
	) -> WIPOffset<UnionWIPOffset>;
}

macro_rules! impl_primitive_int {
	($type:ty, $width:expr, $is_signed:expr) => {
		impl Primitive for $type {
			fn type_discriminant() -> Type {
				Type::Int
			}

			fn type_union<'fbb>(
				builder: &mut FlatBufferBuilder<'fbb>,
			) -> WIPOffset<UnionWIPOffset> {
				Int::create(
					builder,
					&IntArgs {
						bitWidth: $width,
						is_signed: $is_signed,
					},
				)
				.as_union_value()
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
		impl Primitive for $type {
			fn type_discriminant() -> Type {
				Type::FloatingPoint
			}

			fn type_union<'fbb>(
				builder: &mut FlatBufferBuilder<'fbb>,
			) -> WIPOffset<UnionWIPOffset> {
				FloatingPoint::create(
					builder,
					&FloatingPointArgs {
						precision: Precision::$kind,
					},
				)
				.as_union_value()
			}
		}
	};
}
// TODO: feature-gated half
impl_primitive_float!(f32, SINGLE);
impl_primitive_float!(f64, DOUBLE);

pub struct ArrayPrimitive<T: Primitive, V: Validity> {
	validity: V::Container,
	values: Vec<T>,
}

impl<T: Primitive, V: Validity> Array for ArrayPrimitive<T, V> {
	fn len(&self) -> usize {
		self.values.len()
	}

	fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	fn serialize_field<'fbb>(
		&self,
		builder: &mut FlatBufferBuilder<'fbb>,
		name: &str,
	) -> WIPOffset<Field<'fbb>> {
		let type_union = T::type_union(builder);
		let name = builder.create_string(name);

		Field::create(
			builder,
			&FieldArgs {
				name: Some(name),
				nullable: V::IS_NULLABLE,
				type_type: T::type_discriminant(),
				type_: Some(type_union),
				dictionary: None,
				children: None,
				custom_metadata: None,
			},
		)
	}
}

impl<T: Primitive, V: Validity> ArrayPrimitive<T, V> {
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

pub type ArrayU8<V> = ArrayPrimitive<u8, V>;
pub type ArrayU16<V> = ArrayPrimitive<u16, V>;
pub type ArrayU32<V> = ArrayPrimitive<u32, V>;
pub type ArrayU64<V> = ArrayPrimitive<u64, V>;

pub type ArrayI8<V> = ArrayPrimitive<i8, V>;
pub type ArrayI16<V> = ArrayPrimitive<i16, V>;
pub type ArrayI32<V> = ArrayPrimitive<i32, V>;
pub type ArrayI64<V> = ArrayPrimitive<i64, V>;

pub type ArrayF64<V> = ArrayPrimitive<f64, V>;
pub type ArrayF32<V> = ArrayPrimitive<f32, V>;
