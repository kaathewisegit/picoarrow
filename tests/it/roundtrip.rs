use arbitrary::{Arbitrary, Result, Unstructured};
use arbtest::arbtest;
#[cfg(feature = "half")]
use arrow_array::Float16Array;
use arrow_array::{
	Array as _, ArrayRef, BinaryArray, BooleanArray, FixedSizeBinaryArray,
	FixedSizeListArray, Float32Array, Float64Array, Int8Array, Int16Array,
	Int32Array, Int64Array, StringArray, UInt8Array, UInt16Array,
	UInt32Array, UInt64Array,
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
		ArrayI32, ArrayI64, ArrayU8, ArrayU16, ArrayU32, ArrayU64,
		ArrayUtf8, NonNullable,
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
		}

		match *u.choose(&variants)? {
			0 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(bool::arbitrary(u)?);
				}
				Ok(Self::Bool(v))
			}
			1 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(u8::arbitrary(u)?);
				}
				Ok(Self::U8(v))
			}
			2 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(u16::arbitrary(u)?);
				}
				Ok(Self::U16(v))
			}
			3 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(u32::arbitrary(u)?);
				}
				Ok(Self::U32(v))
			}
			4 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(u64::arbitrary(u)?);
				}
				Ok(Self::U64(v))
			}
			5 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(i8::arbitrary(u)?);
				}
				Ok(Self::I8(v))
			}
			6 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(i16::arbitrary(u)?);
				}
				Ok(Self::I16(v))
			}
			7 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(i32::arbitrary(u)?);
				}
				Ok(Self::I32(v))
			}
			8 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(i64::arbitrary(u)?);
				}
				Ok(Self::I64(v))
			}
			9 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(f32::arbitrary(u)?);
				}
				Ok(Self::F32(v))
			}
			10 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(f64::arbitrary(u)?);
				}
				Ok(Self::F64(v))
			}
			#[cfg(feature = "half")]
			11 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(f16::arbitrary(u)?);
				}
				Ok(Self::F16(v))
			}
			12 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					let s_len = u.int_in_range(0..=32)?;
					let bytes = u.bytes(s_len)?;
					v.push(String::from_utf8_lossy(bytes)
						.into_owned());
				}
				Ok(Self::Utf8(v))
			}
			13 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					let b_len = u.int_in_range(0..=32)?;
					let bytes = u.bytes(b_len)?;
					v.push(bytes.to_vec());
				}
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
			_ => unreachable!(),
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
	}
}

fn serialize_batch_stream(batch: &Batch, compression: Compression) -> Vec<u8> {
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
	let mut writer =
		StreamWriter::new(buffer, schema, compression).unwrap();

	let refs: Vec<&dyn Array> =
		arrays.iter().map(|(_, a)| a.as_ref()).collect();

	writer.write_batch(refs).unwrap();
	writer.finish().unwrap()
}

// TODO: deduplicate
fn serialize_batch_file(batch: &Batch, compression: Compression) -> Vec<u8> {
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
	let mut writer = FileWriter::new(buffer, schema, compression).unwrap();

	let refs: Vec<&dyn Array> =
		arrays.iter().map(|(_, a)| a.as_ref()).collect();

	writer.write_batch(refs).unwrap();
	writer.finish().unwrap();
	writer.into_inner()
}

fn deserialize_batch_stream(data: &[u8], original: &Batch) -> Batch {
	let record_batch = ArrowStreamReader::try_new(&mut &data[..], None)
		.unwrap()
		.collect::<Result<Vec<_>, _>>()
		.unwrap()
		.into_iter()
		.next()
		.unwrap();

	let mut arrays = Vec::new();
	for (i, (name, original_arr)) in original.arrays.iter().enumerate() {
		let col = record_batch.column(i);
		let arr = arrow_to_any(col, original_arr);
		arrays.push((name.clone(), arr));
	}
	Batch { arrays }
}

// TODO: deduplicate
fn deserialize_batch_file(data: &[u8], original: &Batch) -> Batch {
	let cursor = Cursor::new(data);
	let record_batch = ArrowFileReader::try_new(cursor, None)
		.unwrap()
		.collect::<Result<Vec<_>, _>>()
		.unwrap()
		.into_iter()
		.next()
		.unwrap();

	let mut arrays = Vec::new();
	for (i, (name, original_arr)) in original.arrays.iter().enumerate() {
		let col = record_batch.column(i);
		let arr = arrow_to_any(col, original_arr);
		arrays.push((name.clone(), arr));
	}
	Batch { arrays }
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
	.size_min(2u32.pow(18))
	.budget_ms(2_000);
}

#[test]
fn roundtrip_stream() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch_stream(batch, Compression::None);
		deserialize_batch_stream(&encoded, batch)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_stream_lz4() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch_stream(batch, Compression::LZ4);
		deserialize_batch_stream(&encoded, batch)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_stream_zstd() {
	check_roundtrip(|batch| {
		let encoded =
			serialize_batch_stream(batch, Compression::Zstd(0));
		deserialize_batch_stream(&encoded, batch)
	});
}

#[test]
fn roundtrip_file() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch_file(batch, Compression::None);
		deserialize_batch_file(&encoded, batch)
	});
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_file_lz4() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch_file(batch, Compression::LZ4);
		deserialize_batch_file(&encoded, batch)
	});
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_file_zstd() {
	check_roundtrip(|batch| {
		let encoded = serialize_batch_file(batch, Compression::Zstd(0));
		deserialize_batch_file(&encoded, batch)
	});
}
