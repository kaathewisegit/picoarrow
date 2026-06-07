use arbitrary::{Arbitrary, Result, Unstructured};
use arbtest::arbtest;
#[cfg(feature = "half")]
use arrow_array::Float16Array;
use arrow_array::{
	Array as _, ArrayRef, BinaryArray, BooleanArray, FixedSizeBinaryArray,
	FixedSizeListArray, Float32Array, Float64Array, Int8Array, Int16Array,
	Int32Array, Int64Array, StringArray, StructArray as ArrowStructArray,
	UInt8Array, UInt16Array, UInt32Array, UInt64Array,
};
use arrow_ipc::reader::{
	FileReader as ArrowFileReader, StreamReader as ArrowStreamReader,
};
#[cfg(feature = "half")]
use half::f16;
#[cfg(feature = "half")]
use picoarrow::array::ArrayF16;
use picoarrow::{
	Schema,
	array::{
		Array, ArrayBinary, ArrayBoolean, ArrayF32, ArrayF64,
		ArrayFixedBinary, ArrayFixedSizeList, ArrayI8, ArrayI16,
		ArrayI32, ArrayI64, ArrayPrimitive, ArrayU8, ArrayU16,
		ArrayU32, ArrayU64, ArrayUtf8, ArrowStruct, NonNullable,
		Nullable,
	},
	ipc::{Compression, FileWriter, StreamWriter},
};

use std::io::Cursor;

#[derive(Debug, Clone)]
pub enum AnyArray {
	Bool(Vec<bool>),
	U8(Vec<u8>),
	U16(Vec<u16>),
	U32(Vec<u32>),
	U64(Vec<u64>),
	I8(Vec<i8>),
	I16(Vec<i16>),
	I32(Vec<i32>),
	I64(Vec<i64>),
	F32(Vec<f32>),
	F64(Vec<f64>),
	#[cfg(feature = "half")]
	F16(Vec<f16>),

	Utf8(Vec<String>),
	Binary(Vec<Vec<u8>>),

	FixedSizeBinary {
		size: u32,
		data: Vec<u8>,
	},

	FixedSizeList {
		size: u32,
		child: Box<AnyArray>,
	},

	NullableBool(Vec<Option<bool>>),
	NullableU8(Vec<Option<u8>>),
	NullableU16(Vec<Option<u16>>),
	NullableU32(Vec<Option<u32>>),
	NullableU64(Vec<Option<u64>>),
	NullableI8(Vec<Option<i8>>),
	NullableI16(Vec<Option<i16>>),
	NullableI32(Vec<Option<i32>>),
	NullableI64(Vec<Option<i64>>),
	NullableF32(Vec<Option<f32>>),
	NullableF64(Vec<Option<f64>>),
	#[cfg(feature = "half")]
	NullableF16(Vec<Option<f16>>),

	NullableFixedSizeBinary {
		size: u32,
		data: Vec<Option<Vec<u8>>>,
	},

	NullableFixedSizeList {
		size: u32,
		child: Box<AnyArray>,
		validity: Vec<bool>,
	},

	Struct {
		names: Vec<String>,
		ids: Vec<i32>,
		scores: Vec<i64>,
	},

	NullableStruct {
		names: Vec<String>,
		ids: Vec<i32>,
		scores: Vec<i64>,
		validity: Vec<bool>,
	},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
	// (name, array) batch tuples
	pub arrays: Vec<(String, AnyArray)>,
}

macro_rules! fsl_to_pico {
	($values:expr, $child_ty:ty, $size:expr) => {{
		let mut list =
			ArrayFixedSizeList::<$child_ty, NonNullable>::new(
				<$child_ty>::new(),
				*$size as i32,
			);
		for chunk in $values.chunks(*$size as usize) {
			list.push(|c: &mut $child_ty| {
				for &val in chunk {
					c.push(val);
				}
			})
			.unwrap();
		}
		Box::new(list) as Box<dyn Array>
	}};
}

macro_rules! push_nullable_primitive {
	($values:expr, $arr_ty:ty) => {{
		let mut arr = <$arr_ty>::new();
		for val in $values {
			match val {
				Some(v) => arr.push(*v),
				None => arr.push_null(),
			}
		}
		Box::new(arr) as Box<dyn Array>
	}};
}

macro_rules! fsl_to_pico_nullable {
	($values:expr, $child_ty:ty, $size:expr, $validity:expr) => {{
		let mut list = ArrayFixedSizeList::<$child_ty, Nullable>::new(
			<$child_ty>::new(),
			*$size as i32,
		);
		for (i, is_valid) in ($validity).iter().enumerate() {
			let start = i * (*$size as usize);
			let end = start + *$size as usize;
			if *is_valid {
				list.push(|c: &mut $child_ty| {
					for &val in &$values[start..end] {
						c.push(val);
					}
				})
				.unwrap();
			} else {
				list.push_null(|c: &mut $child_ty| {
					for &val in &$values[start..end] {
						c.push(val);
					}
				})
				.unwrap();
			}
		}
		Box::new(list) as Box<dyn Array>
	}};
}

macro_rules! arbitrary_primitive {
	($u:expr, $len:expr, $variant:ident) => {{
		let v = (0..$len)
			.map(|_| $u.arbitrary())
			.collect::<Result<_>>()?;
		Ok(Self::$variant(v))
	}};
}

impl AnyArray {
	fn arbitrary_with_len(
		u: &mut Unstructured<'_>,
		len: usize,
		primitive_only: bool,
	) -> Result<Self> {
		let mut variants = Vec::<u8>::from_iter(0..=10);
		if cfg!(feature = "half") {
			variants.push(11)
		};
		if !primitive_only {
			variants.extend(12..=15);
			variants.extend(16..=26);
			if cfg!(feature = "half") {
				variants.push(27);
			}
			variants.extend(28..=29);
			variants.extend(30..=31);
		}

		match *u.choose(&variants)? {
			0 => arbitrary_primitive!(u, len, Bool),
			1 => arbitrary_primitive!(u, len, U8),
			2 => arbitrary_primitive!(u, len, U16),
			3 => arbitrary_primitive!(u, len, U32),
			4 => arbitrary_primitive!(u, len, U64),
			5 => arbitrary_primitive!(u, len, I8),
			6 => arbitrary_primitive!(u, len, I16),
			7 => arbitrary_primitive!(u, len, I32),
			8 => arbitrary_primitive!(u, len, I64),
			9 => arbitrary_primitive!(u, len, F32),
			10 => arbitrary_primitive!(u, len, F64),
			#[cfg(feature = "half")]
			11 => arbitrary_primitive!(u, len, F16),
			12 => {
				let v = (0..len)
					.map(|_| -> Result<_> {
						let s_len =
							u.int_in_range(0..=32)?;
						let bytes = u.bytes(s_len)?;
						Ok(String::from_utf8_lossy(
							bytes,
						)
						.into_owned())
					})
					.collect::<Result<_>>()?;
				Ok(Self::Utf8(v))
			}
			13 => {
				let v = (0..len)
					.map(|_| -> Result<_> {
						let b_len =
							u.int_in_range(0..=32)?;
						Ok(u.bytes(b_len)?.to_vec())
					})
					.collect::<Result<_>>()?;
				Ok(Self::Binary(v))
			}
			14 => {
				let size = u.int_in_range(1..=8)?;
				let child_len = size as usize * len;
				let child = Self::arbitrary_with_len(
					u, child_len, true,
				)?;
				Ok(Self::FixedSizeList {
					size,
					child: child.into(),
				})
			}
			15 => {
				let size = u.int_in_range(1..=50)?;
				let total = size as usize * len;
				let data = u.bytes(total)?.to_vec();
				Ok(Self::FixedSizeBinary { size, data })
			}
			16 => arbitrary_primitive!(u, len, NullableBool),
			17 => arbitrary_primitive!(u, len, NullableU8),
			18 => arbitrary_primitive!(u, len, NullableU16),
			19 => arbitrary_primitive!(u, len, NullableU32),
			20 => arbitrary_primitive!(u, len, NullableU64),
			21 => arbitrary_primitive!(u, len, NullableI8),
			22 => arbitrary_primitive!(u, len, NullableI16),
			23 => arbitrary_primitive!(u, len, NullableI32),
			24 => arbitrary_primitive!(u, len, NullableI64),
			25 => arbitrary_primitive!(u, len, NullableF32),
			26 => arbitrary_primitive!(u, len, NullableF64),
			#[cfg(feature = "half")]
			27 => arbitrary_primitive!(u, len, NullableF16),
			28 => {
				let size = u.int_in_range(1..=50)?;
				let data = (0..len)
					.map(|_| -> Result<_> {
						Ok(if u.arbitrary()? {
							None
						} else {
							Some(u.bytes(
								size as usize
							)?
							.to_vec())
						})
					})
					.collect::<Result<_>>()?;
				Ok(Self::NullableFixedSizeBinary { size, data })
			}
			29 => {
				let size = u.int_in_range(1..=8)?;
				let child_len = size as usize * len;
				let child = Self::arbitrary_with_len(
					u, child_len, true,
				)?;
				let validity = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::NullableFixedSizeList {
					size,
					child: child.into(),
					validity,
				})
			}
			30 => {
				let ids = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let scores = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::Struct {
					names: vec![
						"id".into(),
						"score".into(),
					],
					ids,
					scores,
				})
			}
			31 => {
				let ids = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let scores = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let validity = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::NullableStruct {
					names: vec![
						"id".into(),
						"score".into(),
					],
					ids,
					scores,
					validity,
				})
			}
			_ => unreachable!(),
		}
	}

	fn new_data(
		&self,
		u: &mut Unstructured<'_>,
		len: usize,
	) -> Result<Self> {
		match self {
			Self::Bool(_) => arbitrary_primitive!(u, len, Bool),
			Self::U8(_) => arbitrary_primitive!(u, len, U8),
			Self::U16(_) => arbitrary_primitive!(u, len, U16),
			Self::U32(_) => arbitrary_primitive!(u, len, U32),
			Self::U64(_) => arbitrary_primitive!(u, len, U64),
			Self::I8(_) => arbitrary_primitive!(u, len, I8),
			Self::I16(_) => arbitrary_primitive!(u, len, I16),
			Self::I32(_) => arbitrary_primitive!(u, len, I32),
			Self::I64(_) => arbitrary_primitive!(u, len, I64),
			Self::F32(_) => arbitrary_primitive!(u, len, F32),
			Self::F64(_) => arbitrary_primitive!(u, len, F64),
			#[cfg(feature = "half")]
			Self::F16(_) => arbitrary_primitive!(u, len, F16),
			Self::Utf8(_) => {
				let v = (0..len)
					.map(|_| -> Result<_> {
						let s_len =
							u.int_in_range(0..=32)?;
						let bytes = u.bytes(s_len)?;
						Ok(String::from_utf8_lossy(
							bytes,
						)
						.into_owned())
					})
					.collect::<Result<_>>()?;
				Ok(Self::Utf8(v))
			}
			Self::Binary(_) => {
				let v = (0..len)
					.map(|_| -> Result<_> {
						let b_len =
							u.int_in_range(0..=32)?;
						Ok(u.bytes(b_len)?.to_vec())
					})
					.collect::<Result<_>>()?;
				Ok(Self::Binary(v))
			}
			Self::FixedSizeBinary { size, .. } => {
				let total = *size as usize * len;
				let data = u.bytes(total)?.to_vec();
				Ok(Self::FixedSizeBinary { size: *size, data })
			}
			Self::FixedSizeList { size, child, .. } => {
				let child_len = *size as usize * len;
				let new_child = child.new_data(u, child_len)?;
				Ok(Self::FixedSizeList {
					size: *size,
					child: Box::new(new_child),
				})
			}
			Self::NullableBool(_) => {
				arbitrary_primitive!(u, len, NullableBool)
			}
			Self::NullableU8(_) => {
				arbitrary_primitive!(u, len, NullableU8)
			}
			Self::NullableU16(_) => {
				arbitrary_primitive!(u, len, NullableU16)
			}
			Self::NullableU32(_) => {
				arbitrary_primitive!(u, len, NullableU32)
			}
			Self::NullableU64(_) => {
				arbitrary_primitive!(u, len, NullableU64)
			}
			Self::NullableI8(_) => {
				arbitrary_primitive!(u, len, NullableI8)
			}
			Self::NullableI16(_) => {
				arbitrary_primitive!(u, len, NullableI16)
			}
			Self::NullableI32(_) => {
				arbitrary_primitive!(u, len, NullableI32)
			}
			Self::NullableI64(_) => {
				arbitrary_primitive!(u, len, NullableI64)
			}
			Self::NullableF32(_) => {
				arbitrary_primitive!(u, len, NullableF32)
			}
			Self::NullableF64(_) => {
				arbitrary_primitive!(u, len, NullableF64)
			}
			#[cfg(feature = "half")]
			Self::NullableF16(_) => {
				arbitrary_primitive!(u, len, NullableF16)
			}
			Self::NullableFixedSizeBinary { size, .. } => {
				let data = (0..len)
					.map(|_| -> Result<_> {
						Ok(if u.arbitrary()? {
							None
						} else {
							Some(u.bytes(
								*size as usize
							)?
							.to_vec())
						})
					})
					.collect::<Result<_>>()?;
				Ok(Self::NullableFixedSizeBinary {
					size: *size,
					data,
				})
			}
			Self::NullableFixedSizeList { size, child, .. } => {
				let child_len = *size as usize * len;
				let new_child = child.new_data(u, child_len)?;
				let validity = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::NullableFixedSizeList {
					size: *size,
					child: Box::new(new_child),
					validity,
				})
			}
			Self::Struct { names, .. } => {
				let ids = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let scores = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::Struct {
					names: names.clone(),
					ids,
					scores,
				})
			}
			Self::NullableStruct { names, .. } => {
				let ids = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let scores = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				let validity = (0..len)
					.map(|_| u.arbitrary())
					.collect::<Result<_>>()?;
				Ok(Self::NullableStruct {
					names: names.clone(),
					ids,
					scores,
					validity,
				})
			}
		}
	}

	fn to_picoarray(&self) -> Box<dyn Array> {
		macro_rules! push_primitive {
			($values:expr, $arr_ty:ty) => {{
				let mut arr = <$arr_ty>::new();
				for &val in $values {
					arr.push(val);
				}
				Box::new(arr) as Box<dyn Array>
			}};
		}

		match self {
			Self::Bool(v) => {
				push_primitive!(v, ArrayBoolean<NonNullable>)
			}
			Self::U8(v) => {
				push_primitive!(v, ArrayU8<NonNullable>)
			}
			Self::U16(v) => {
				push_primitive!(v, ArrayU16<NonNullable>)
			}
			Self::U32(v) => {
				push_primitive!(v, ArrayU32<NonNullable>)
			}
			Self::U64(v) => {
				push_primitive!(v, ArrayU64<NonNullable>)
			}
			Self::I8(v) => {
				push_primitive!(v, ArrayI8<NonNullable>)
			}
			Self::I16(v) => {
				push_primitive!(v, ArrayI16<NonNullable>)
			}
			Self::I32(v) => {
				push_primitive!(v, ArrayI32<NonNullable>)
			}
			Self::I64(v) => {
				push_primitive!(v, ArrayI64<NonNullable>)
			}
			Self::F32(v) => {
				push_primitive!(v, ArrayF32<NonNullable>)
			}
			Self::F64(v) => {
				push_primitive!(v, ArrayF64<NonNullable>)
			}
			#[cfg(feature = "half")]
			Self::F16(v) => {
				push_primitive!(v, ArrayF16<NonNullable>)
			}
			Self::Utf8(v) => {
				let mut arr = ArrayUtf8::<NonNullable>::new();
				for s in v {
					arr.push(s).unwrap();
				}
				Box::new(arr)
			}
			Self::Binary(v) => {
				let mut arr = ArrayBinary::<NonNullable>::new();
				for b in v {
					arr.push(b).unwrap();
				}
				Box::new(arr)
			}
			Self::FixedSizeBinary { size, data } => {
				let mut arr =
					ArrayFixedBinary::<NonNullable>::new(
						*size as i32,
					);
				for chunk in data.chunks(*size as usize) {
					arr.push(chunk).unwrap();
				}
				Box::new(arr)
			}
			Self::FixedSizeList { size, child } => {
				match child.as_ref() {
					AnyArray::Bool(v) => fsl_to_pico!(
						v,
						ArrayBoolean<NonNullable>,
						size
					),
					AnyArray::U8(v) => fsl_to_pico!(
						v,
						ArrayU8<NonNullable>,
						size
					),
					AnyArray::U16(v) => fsl_to_pico!(
						v,
						ArrayU16<NonNullable>,
						size
					),
					AnyArray::U32(v) => fsl_to_pico!(
						v,
						ArrayU32<NonNullable>,
						size
					),
					AnyArray::U64(v) => fsl_to_pico!(
						v,
						ArrayU64<NonNullable>,
						size
					),
					AnyArray::I8(v) => fsl_to_pico!(
						v,
						ArrayI8<NonNullable>,
						size
					),
					AnyArray::I16(v) => fsl_to_pico!(
						v,
						ArrayI16<NonNullable>,
						size
					),
					AnyArray::I32(v) => fsl_to_pico!(
						v,
						ArrayI32<NonNullable>,
						size
					),
					AnyArray::I64(v) => fsl_to_pico!(
						v,
						ArrayI64<NonNullable>,
						size
					),
					AnyArray::F32(v) => fsl_to_pico!(
						v,
						ArrayF32<NonNullable>,
						size
					),
					AnyArray::F64(v) => fsl_to_pico!(
						v,
						ArrayF64<NonNullable>,
						size
					),
					#[cfg(feature = "half")]
					AnyArray::F16(v) => fsl_to_pico!(
						v,
						ArrayF16<NonNullable>,
						size
					),
					_ => panic!(
						"unsupported FixedSizeList child {child:?}"
					),
				}
			}
			Self::NullableBool(v) => {
				push_nullable_primitive!(
					v,
					ArrayBoolean<Nullable>
				)
			}
			Self::NullableU8(v) => {
				push_nullable_primitive!(v, ArrayU8<Nullable>)
			}
			Self::NullableU16(v) => {
				push_nullable_primitive!(v, ArrayU16<Nullable>)
			}
			Self::NullableU32(v) => {
				push_nullable_primitive!(v, ArrayU32<Nullable>)
			}
			Self::NullableU64(v) => {
				push_nullable_primitive!(v, ArrayU64<Nullable>)
			}
			Self::NullableI8(v) => {
				push_nullable_primitive!(v, ArrayI8<Nullable>)
			}
			Self::NullableI16(v) => {
				push_nullable_primitive!(v, ArrayI16<Nullable>)
			}
			Self::NullableI32(v) => {
				push_nullable_primitive!(v, ArrayI32<Nullable>)
			}
			Self::NullableI64(v) => {
				push_nullable_primitive!(v, ArrayI64<Nullable>)
			}
			Self::NullableF32(v) => {
				push_nullable_primitive!(v, ArrayF32<Nullable>)
			}
			Self::NullableF64(v) => {
				push_nullable_primitive!(v, ArrayF64<Nullable>)
			}
			#[cfg(feature = "half")]
			Self::NullableF16(v) => {
				push_nullable_primitive!(v, ArrayF16<Nullable>)
			}
			Self::NullableFixedSizeBinary { size, data } => {
				let mut arr = ArrayFixedBinary::<Nullable>::new(
					*size as i32,
				);
				for val in data {
					match val {
						Some(bytes) => {
							arr.push(bytes)
								.unwrap();
						}
						None => arr.push_null(),
					}
				}
				Box::new(arr)
			}
			Self::NullableFixedSizeList {
				size,
				child,
				validity,
			} => match child.as_ref() {
				AnyArray::Bool(v) => fsl_to_pico_nullable!(
					v,
					ArrayBoolean<NonNullable>,
					size,
					validity
				),
				AnyArray::U8(v) => fsl_to_pico_nullable!(
					v,
					ArrayU8<NonNullable>,
					size,
					validity
				),
				AnyArray::U16(v) => fsl_to_pico_nullable!(
					v,
					ArrayU16<NonNullable>,
					size,
					validity
				),
				AnyArray::U32(v) => fsl_to_pico_nullable!(
					v,
					ArrayU32<NonNullable>,
					size,
					validity
				),
				AnyArray::U64(v) => fsl_to_pico_nullable!(
					v,
					ArrayU64<NonNullable>,
					size,
					validity
				),
				AnyArray::I8(v) => fsl_to_pico_nullable!(
					v,
					ArrayI8<NonNullable>,
					size,
					validity
				),
				AnyArray::I16(v) => fsl_to_pico_nullable!(
					v,
					ArrayI16<NonNullable>,
					size,
					validity
				),
				AnyArray::I32(v) => fsl_to_pico_nullable!(
					v,
					ArrayI32<NonNullable>,
					size,
					validity
				),
				AnyArray::I64(v) => fsl_to_pico_nullable!(
					v,
					ArrayI64<NonNullable>,
					size,
					validity
				),
				AnyArray::F32(v) => fsl_to_pico_nullable!(
					v,
					ArrayF32<NonNullable>,
					size,
					validity
				),
				AnyArray::F64(v) => fsl_to_pico_nullable!(
					v,
					ArrayF64<NonNullable>,
					size,
					validity
				),
				#[cfg(feature = "half")]
				AnyArray::F16(v) => fsl_to_pico_nullable!(
					v,
					ArrayF16<NonNullable>,
					size,
					validity
				),
				_ => panic!(
					"unsupported NullableFixedSizeList child {child:?}"
				),
			},
			Self::Struct { names, ids, scores } => {
				type Fields = (
					ArrayPrimitive<i32, NonNullable>,
					ArrayPrimitive<i64, NonNullable>,
				);
				let mut arr: ArrowStruct<Fields, NonNullable> =
					ArrowStruct::new(
						names.clone(),
						(
							ArrayPrimitive::new(),
							ArrayPrimitive::new(),
						),
					);
				for (&id, &score) in
					ids.iter().zip(scores.iter())
				{
					arr.push(|fields| {
						fields.0.push(id);
						fields.1.push(score);
					})
					.unwrap();
				}
				Box::new(arr)
			}
			Self::NullableStruct {
				names,
				ids,
				scores,
				validity,
			} => {
				type Fields = (
					ArrayPrimitive<i32, NonNullable>,
					ArrayPrimitive<i64, NonNullable>,
				);
				let mut arr: ArrowStruct<Fields, Nullable> =
					ArrowStruct::new(
						names.clone(),
						(
							ArrayPrimitive::new(),
							ArrayPrimitive::new(),
						),
					);
				for (i, (&id, &score)) in ids
					.iter()
					.zip(scores.iter())
					.enumerate()
				{
					if validity[i] {
						arr.push(|fields| {
							fields.0.push(id);
							fields.1.push(score);
						})
						.unwrap();
					} else {
						arr.push_null(|fields| {
							fields.0.push(id);
							fields.1.push(score);
						})
						.unwrap();
					}
				}
				Box::new(arr)
			}
		}
	}
}

impl PartialEq for AnyArray {
	fn eq(&self, other: &Self) -> bool {
		macro_rules! floats_eq {
			($a:expr, $b:expr) => {
				$a.len() == $b.len()
					&& $a.iter().zip($b.iter()).all(
						|(x, y)| {
							x.to_bits()
								== y.to_bits()
						},
					)
			};
		}

		macro_rules! nullable_floats_eq {
			($a:expr, $b:expr) => {
				$a.len() == $b.len()
					&& $a.iter().zip($b.iter()).all(
						|(x, y)| match (x, y) {
							(None, None) => true,
							(Some(a), Some(b)) => a
								.to_bits()
								== b.to_bits(),
							_ => false,
						},
					)
			};
		}

		match (self, other) {
			(Self::Bool(a), Self::Bool(b)) => a == b,
			(Self::U8(a), Self::U8(b)) => a == b,
			(Self::U16(a), Self::U16(b)) => a == b,
			(Self::U32(a), Self::U32(b)) => a == b,
			(Self::U64(a), Self::U64(b)) => a == b,
			(Self::I8(a), Self::I8(b)) => a == b,
			(Self::I16(a), Self::I16(b)) => a == b,
			(Self::I32(a), Self::I32(b)) => a == b,
			(Self::I64(a), Self::I64(b)) => a == b,
			(Self::F32(a), Self::F32(b)) => floats_eq!(a, b),
			(Self::F64(a), Self::F64(b)) => floats_eq!(a, b),
			#[cfg(feature = "half")]
			(Self::F16(a), Self::F16(b)) => floats_eq!(a, b),

			(Self::Utf8(a), Self::Utf8(b)) => a == b,
			(Self::Binary(a), Self::Binary(b)) => a == b,
			(
				Self::FixedSizeBinary { size: s1, data: d1 },
				Self::FixedSizeBinary { size: s2, data: d2 },
			) => s1 == s2 && d1 == d2,
			(
				Self::FixedSizeList {
					size: s1,
					child: c1,
				},
				Self::FixedSizeList {
					size: s2,
					child: c2,
				},
			) => s1 == s2 && c1 == c2,

			(Self::NullableBool(a), Self::NullableBool(b)) => {
				a == b
			}
			(Self::NullableU8(a), Self::NullableU8(b)) => a == b,
			(Self::NullableU16(a), Self::NullableU16(b)) => a == b,
			(Self::NullableU32(a), Self::NullableU32(b)) => a == b,
			(Self::NullableU64(a), Self::NullableU64(b)) => a == b,
			(Self::NullableI8(a), Self::NullableI8(b)) => a == b,
			(Self::NullableI16(a), Self::NullableI16(b)) => a == b,
			(Self::NullableI32(a), Self::NullableI32(b)) => a == b,
			(Self::NullableI64(a), Self::NullableI64(b)) => a == b,
			(Self::NullableF32(a), Self::NullableF32(b)) => {
				nullable_floats_eq!(a, b)
			}
			(Self::NullableF64(a), Self::NullableF64(b)) => {
				nullable_floats_eq!(a, b)
			}
			#[cfg(feature = "half")]
			(Self::NullableF16(a), Self::NullableF16(b)) => {
				nullable_floats_eq!(a, b)
			}
			(
				Self::NullableFixedSizeBinary {
					size: s1,
					data: d1,
				},
				Self::NullableFixedSizeBinary {
					size: s2,
					data: d2,
				},
			) => s1 == s2 && d1 == d2,
			(
				Self::NullableFixedSizeList {
					size: s1,
					child: c1,
					validity: v1,
				},
				Self::NullableFixedSizeList {
					size: s2,
					child: c2,
					validity: v2,
				},
			) => s1 == s2 && c1 == c2 && v1 == v2,
			(
				Self::Struct {
					names: n1,
					ids: i1,
					scores: s1,
				},
				Self::Struct {
					names: n2,
					ids: i2,
					scores: s2,
				},
			) => n1 == n2 && i1 == i2 && s1 == s2,
			(
				Self::NullableStruct {
					names: n1,
					ids: i1,
					scores: s1,
					validity: v1,
				},
				Self::NullableStruct {
					names: n2,
					ids: i2,
					scores: s2,
					validity: v2,
				},
			) => n1 == n2 && i1 == i2 && s1 == s2 && v1 == v2,
			_ => false,
		}
	}
}

impl Eq for AnyArray {}

impl<'a> Arbitrary<'a> for Batch {
	fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
		let num_cols = u.int_in_range(0..=10)?;
		let len = u.int_in_range(0..=100)?;

		let mut arrays = Vec::new();
		for _ in 0..num_cols {
			let name = u.arbitrary()?;
			let arr = AnyArray::arbitrary_with_len(u, len, false)?;
			arrays.push((name, arr));
		}

		Ok(Self { arrays })
	}
}

fn arrow_to_any(col: &ArrayRef, original: &AnyArray) -> AnyArray {
	macro_rules! downcast_primitive {
		($arr_ty:ty, $variant:ident) => {{
			let arr =
				col.as_any().downcast_ref::<$arr_ty>().unwrap();
			AnyArray::$variant(
				(0..arr.len()).map(|i| arr.value(i)).collect(),
			)
		}};
	}

	macro_rules! downcast_nullable_primitive {
		($arr_ty:ty, $variant:ident) => {{
			let arr =
				col.as_any().downcast_ref::<$arr_ty>().unwrap();
			AnyArray::$variant(
				(0..arr.len())
					.map(|i| {
						if arr.is_null(i) {
							None
						} else {
							Some(arr.value(i))
						}
					})
					.collect(),
			)
		}};
	}

	match original {
		AnyArray::Bool(_) => {
			downcast_primitive!(BooleanArray, Bool)
		}
		AnyArray::U8(_) => {
			downcast_primitive!(UInt8Array, U8)
		}
		AnyArray::U16(_) => {
			downcast_primitive!(UInt16Array, U16)
		}
		AnyArray::U32(_) => {
			downcast_primitive!(UInt32Array, U32)
		}
		AnyArray::U64(_) => {
			downcast_primitive!(UInt64Array, U64)
		}
		AnyArray::I8(_) => {
			downcast_primitive!(Int8Array, I8)
		}
		AnyArray::I16(_) => {
			downcast_primitive!(Int16Array, I16)
		}
		AnyArray::I32(_) => {
			downcast_primitive!(Int32Array, I32)
		}
		AnyArray::I64(_) => {
			downcast_primitive!(Int64Array, I64)
		}
		AnyArray::F32(_) => {
			downcast_primitive!(Float32Array, F32)
		}
		AnyArray::F64(_) => {
			downcast_primitive!(Float64Array, F64)
		}
		#[cfg(feature = "half")]
		AnyArray::F16(_) => {
			downcast_primitive!(Float16Array, F16)
		}
		AnyArray::Utf8(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<StringArray>()
				.unwrap();
			AnyArray::Utf8(
				(0..arr.len())
					.map(|i| arr.value(i).to_owned())
					.collect(),
			)
		}
		AnyArray::Binary(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<BinaryArray>()
				.unwrap();
			AnyArray::Binary(
				(0..arr.len())
					.map(|i| arr.value(i).to_vec())
					.collect(),
			)
		}
		AnyArray::FixedSizeBinary { size, .. } => {
			let arr = col
				.as_any()
				.downcast_ref::<FixedSizeBinaryArray>()
				.unwrap();
			assert_eq!(arr.value_length() as u32, *size);
			let mut data =
				Vec::with_capacity(arr.len() * *size as usize);
			for i in 0..arr.len() {
				data.extend_from_slice(arr.value(i));
			}
			AnyArray::FixedSizeBinary { size: *size, data }
		}
		AnyArray::FixedSizeList { size, child } => {
			let list_arr = col
				.as_any()
				.downcast_ref::<FixedSizeListArray>()
				.unwrap();
			assert_eq!(list_arr.value_length() as u32, *size);
			let child_col = list_arr.values();
			let inner = arrow_to_any(child_col, child);
			AnyArray::FixedSizeList {
				size: *size,
				child: Box::new(inner),
			}
		}
		AnyArray::NullableBool(_) => {
			downcast_nullable_primitive!(BooleanArray, NullableBool)
		}
		AnyArray::NullableU8(_) => {
			downcast_nullable_primitive!(UInt8Array, NullableU8)
		}
		AnyArray::NullableU16(_) => {
			downcast_nullable_primitive!(UInt16Array, NullableU16)
		}
		AnyArray::NullableU32(_) => {
			downcast_nullable_primitive!(UInt32Array, NullableU32)
		}
		AnyArray::NullableU64(_) => {
			downcast_nullable_primitive!(UInt64Array, NullableU64)
		}
		AnyArray::NullableI8(_) => {
			downcast_nullable_primitive!(Int8Array, NullableI8)
		}
		AnyArray::NullableI16(_) => {
			downcast_nullable_primitive!(Int16Array, NullableI16)
		}
		AnyArray::NullableI32(_) => {
			downcast_nullable_primitive!(Int32Array, NullableI32)
		}
		AnyArray::NullableI64(_) => {
			downcast_nullable_primitive!(Int64Array, NullableI64)
		}
		AnyArray::NullableF32(_) => {
			downcast_nullable_primitive!(Float32Array, NullableF32)
		}
		AnyArray::NullableF64(_) => {
			downcast_nullable_primitive!(Float64Array, NullableF64)
		}
		#[cfg(feature = "half")]
		AnyArray::NullableF16(_) => {
			downcast_nullable_primitive!(Float16Array, NullableF16)
		}
		AnyArray::NullableFixedSizeBinary { size, .. } => {
			let arr = col
				.as_any()
				.downcast_ref::<FixedSizeBinaryArray>()
				.unwrap();
			assert_eq!(arr.value_length() as u32, *size);
			let data: Vec<Option<Vec<u8>>> = (0..arr.len())
				.map(|i| {
					if arr.is_null(i) {
						None
					} else {
						Some(arr.value(i).to_vec())
					}
				})
				.collect();
			AnyArray::NullableFixedSizeBinary { size: *size, data }
		}
		AnyArray::NullableFixedSizeList { size, child, .. } => {
			let list_arr = col
				.as_any()
				.downcast_ref::<FixedSizeListArray>()
				.unwrap();
			assert_eq!(list_arr.value_length() as u32, *size);
			let validity: Vec<bool> = (0..list_arr.len())
				.map(|i| !list_arr.is_null(i))
				.collect();
			let child_col = list_arr.values();
			let inner = arrow_to_any(child_col, child);
			AnyArray::NullableFixedSizeList {
				size: *size,
				child: Box::new(inner),
				validity,
			}
		}
		AnyArray::Struct { names, .. } => {
			let struct_arr = col
				.as_any()
				.downcast_ref::<ArrowStructArray>()
				.unwrap();
			let id_arr = struct_arr.column(0);
			let score_arr = struct_arr.column(1);
			let ids = (0..struct_arr.len())
				.map(|i| {
					id_arr.as_any()
						.downcast_ref::<Int32Array>()
						.unwrap()
						.value(i)
				})
				.collect();
			let scores = (0..struct_arr.len())
				.map(|i| {
					score_arr
						.as_any()
						.downcast_ref::<Int64Array>()
						.unwrap()
						.value(i)
				})
				.collect();
			AnyArray::Struct {
				names: names.clone(),
				ids,
				scores,
			}
		}
		AnyArray::NullableStruct { names, .. } => {
			let struct_arr = col
				.as_any()
				.downcast_ref::<ArrowStructArray>()
				.unwrap();
			let id_arr = struct_arr.column(0);
			let score_arr = struct_arr.column(1);
			let ids = (0..struct_arr.len())
				.map(|i| {
					id_arr.as_any()
						.downcast_ref::<Int32Array>()
						.unwrap()
						.value(i)
				})
				.collect();
			let scores = (0..struct_arr.len())
				.map(|i| {
					score_arr
						.as_any()
						.downcast_ref::<Int64Array>()
						.unwrap()
						.value(i)
				})
				.collect();
			let validity: Vec<bool> = (0..struct_arr.len())
				.map(|i| !struct_arr.is_null(i))
				.collect();
			AnyArray::NullableStruct {
				names: names.clone(),
				ids,
				scores,
				validity,
			}
		}
	}
}

enum IpcFormat {
	Stream,
	File,
}

fn serialize_batch(
	batch: &Batch,
	compression: Compression,
	format: IpcFormat,
) -> Vec<u8> {
	let arrays: Vec<(String, Box<dyn Array>)> = batch
		.arrays
		.iter()
		.map(|(name, arr)| (name.clone(), arr.to_picoarray()))
		.collect();

	let schema = Schema::from_arrays(
		arrays.iter()
			.map(|(name, arr)| (name.as_str(), arr.as_ref())),
	);

	let buffer = Vec::new();

	let refs: Vec<&dyn Array> =
		arrays.iter().map(|(_, a)| a.as_ref()).collect();

	match format {
		IpcFormat::Stream => {
			let mut writer =
				StreamWriter::new(buffer, schema, compression)
					.unwrap();
			writer.write_batch(refs).unwrap();
			writer.finish().unwrap()
		}
		IpcFormat::File => {
			let mut writer =
				FileWriter::new(buffer, schema, compression)
					.unwrap();
			writer.write_batch(refs).unwrap();
			writer.finish().unwrap();
			writer.into_inner()
		}
	}
}

fn serialize_batches(
	batches: &[Batch],
	compression: Compression,
	format: IpcFormat,
) -> Vec<u8> {
	assert!(!batches.is_empty());
	let first = &batches[0];
	let arrays: Vec<(String, Box<dyn Array>)> = first
		.arrays
		.iter()
		.map(|(name, arr)| (name.clone(), arr.to_picoarray()))
		.collect();

	let schema = Schema::from_arrays(
		arrays.iter()
			.map(|(name, arr)| (name.as_str(), arr.as_ref())),
	);

	let buffer = Vec::new();

	match format {
		IpcFormat::Stream => {
			let mut writer =
				StreamWriter::new(buffer, schema, compression)
					.unwrap();
			for batch in batches {
				let converted: Vec<(String, Box<dyn Array>)> =
					batch.arrays
						.iter()
						.map(|(name, arr)| {
							(name.clone(), arr.to_picoarray())
						})
						.collect();
				let refs: Vec<&dyn Array> = converted
					.iter()
					.map(|(_, a)| a.as_ref())
					.collect();
				writer.write_batch(refs).unwrap();
			}
			writer.finish().unwrap()
		}
		IpcFormat::File => {
			let mut writer =
				FileWriter::new(buffer, schema, compression)
					.unwrap();
			for batch in batches {
				let converted: Vec<(String, Box<dyn Array>)> =
					batch.arrays
						.iter()
						.map(|(name, arr)| {
							(name.clone(), arr.to_picoarray())
						})
						.collect();
				let refs: Vec<&dyn Array> = converted
					.iter()
					.map(|(_, a)| a.as_ref())
					.collect();
				writer.write_batch(refs).unwrap();
			}
			writer.finish().unwrap();
			writer.into_inner()
		}
	}
}

use arrow_array::RecordBatch as ArrowRecordBatch;

fn deserialize_record_batch(
	record_batch: &ArrowRecordBatch,
	original: &Batch,
) -> Batch {
	let mut arrays = Vec::new();
	for (i, (name, original_arr)) in original.arrays.iter().enumerate() {
		let col = record_batch.column(i);
		let arr = arrow_to_any(col, original_arr);
		arrays.push((name.clone(), arr));
	}
	Batch { arrays }
}

fn deserialize_batches(
	data: &[u8],
	originals: &[Batch],
	format: IpcFormat,
) -> Vec<Batch> {
	let record_batches: Vec<ArrowRecordBatch> = match format {
		IpcFormat::Stream => {
			ArrowStreamReader::try_new(&mut &data[..], None)
				.unwrap()
				.collect::<Result<Vec<_>, _>>()
				.unwrap()
		}
		IpcFormat::File => {
			ArrowFileReader::try_new(Cursor::new(data), None)
				.unwrap()
				.collect::<Result<Vec<_>, _>>()
				.unwrap()
		}
	};

	record_batches
		.into_iter()
		.zip(originals)
		.map(|(rb, original)| deserialize_record_batch(&rb, original))
		.collect()
}

fn deserialize_batch(
	data: &[u8],
	original: &Batch,
	format: IpcFormat,
) -> Batch {
	deserialize_batches(data, std::slice::from_ref(original), format)
		.into_iter()
		.next()
		.unwrap()
}

fn check_roundtrip(f: impl Fn(&Batch) -> Batch) {
	arbtest(|u| {
		let batch = Batch::arbitrary(u)?;

		if batch.arrays.is_empty() {
			return Ok(());
		}

		let decoded = f(&batch);

		assert_eq!(
			batch, decoded,
			"roundtrip failed\nbatch:\n{batch:?}\ndecoded\n{decoded:?}"
		);
		Ok(())
	})
	.size_min(2u32.pow(18));
}

fn check_roundtrip_multi(f: impl Fn(&[Batch]) -> Vec<Batch>) {
	arbtest(|u| {
		let num_batches = u.int_in_range(1..=5)?;

		let first = Batch::arbitrary(u)?;
		if first.arrays.is_empty() {
			return Ok(());
		}

		let mut batches = vec![first];
		for _ in 1..num_batches {
			let len = u.int_in_range(0..=50)?;
			let mut arrays = Vec::new();
			for (name, template) in &batches[0].arrays {
				let arr = template.new_data(u, len)?;
				arrays.push((name.clone(), arr));
			}
			batches.push(Batch { arrays });
		}

		let decoded = f(&batches);

		assert_eq!(
			batches, decoded,
			"roundtrip failed\nbatches:\n{batches:?}\ndecoded\n{decoded:?}"
		);
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn roundtrip_stream() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::None,
			IpcFormat::Stream,
		);
		deserialize_batch(&encoded, batch, IpcFormat::Stream)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_stream_lz4() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::LZ4,
			IpcFormat::Stream,
		);
		deserialize_batch(&encoded, batch, IpcFormat::Stream)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_stream_zstd() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::Zstd(0),
			IpcFormat::Stream,
		);
		deserialize_batch(&encoded, batch, IpcFormat::Stream)
	});
}

#[test]
fn roundtrip_file() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::None,
			IpcFormat::File,
		);
		deserialize_batch(&encoded, batch, IpcFormat::File)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_file_lz4() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::LZ4,
			IpcFormat::File,
		);
		deserialize_batch(&encoded, batch, IpcFormat::File)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_file_zstd() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch(
			batch,
			Compression::Zstd(0),
			IpcFormat::File,
		);
		deserialize_batch(&encoded, batch, IpcFormat::File)
	});
}

#[test]
fn roundtrip_multi_stream() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::None,
			IpcFormat::Stream,
		);
		deserialize_batches(&encoded, batches, IpcFormat::Stream)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_multi_stream_lz4() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::LZ4,
			IpcFormat::Stream,
		);
		deserialize_batches(&encoded, batches, IpcFormat::Stream)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_multi_stream_zstd() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::Zstd(0),
			IpcFormat::Stream,
		);
		deserialize_batches(&encoded, batches, IpcFormat::Stream)
	});
}

#[test]
fn roundtrip_multi_file() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::None,
			IpcFormat::File,
		);
		deserialize_batches(&encoded, batches, IpcFormat::File)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_multi_file_lz4() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::LZ4,
			IpcFormat::File,
		);
		deserialize_batches(&encoded, batches, IpcFormat::File)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_multi_file_zstd() {
	check_roundtrip_multi(|batches| {
		let encoded = serialize_batches(
			batches,
			Compression::Zstd(0),
			IpcFormat::File,
		);
		deserialize_batches(&encoded, batches, IpcFormat::File)
	});
}
