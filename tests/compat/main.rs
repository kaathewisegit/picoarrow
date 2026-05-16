mod stream;

use arbitrary::{Arbitrary, Result, Unstructured};
use arbtest::arbtest;
use arrow_array::{
	Array as _, ArrayRef, BinaryArray, BooleanArray, FixedSizeListArray,
	Float32Array, Float64Array, StringArray, UInt8Array,
};
use arrow_ipc::reader::{
	FileReader as ArrowFileReader, StreamReader as ArrowStreamReader,
};
use picoarrow::{
	Schema,
	array::{
		Array, ArrayBinary, ArrayBoolean, ArrayF32, ArrayF64,
		ArrayFixedSizeList, ArrayU8, ArrayUtf8, NonNullable,
	},
	ipc::{Compression, FileWriter, StreamWriter},
};

use std::io::Cursor;

#[derive(Debug, Clone)]
pub enum AnyArray {
	Bool(Vec<bool>),
	U8(Vec<u8>),
	// TODO: other integer variants
	F32(Vec<f32>),
	F64(Vec<f64>),

	Utf8(Vec<String>),
	Binary(Vec<Vec<u8>>),

	FixedSizeList { size: u32, child: Box<AnyArray> },
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
		let end = if primitive_only { 3 } else { 6 };
		let variant: u8 = u.int_in_range(0..=end)?;
		match variant {
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
					v.push(f32::arbitrary(u)?);
				}
				Ok(Self::F32(v))
			}
			3 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					v.push(f64::arbitrary(u)?);
				}
				Ok(Self::F64(v))
			}
			4 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					let s_len = u.int_in_range(0..=32)?;
					let bytes = u.bytes(s_len)?;
					v.push(String::from_utf8_lossy(bytes)
						.into_owned());
				}
				Ok(Self::Utf8(v))
			}
			5 => {
				let mut v = Vec::with_capacity(len);
				for _ in 0..len {
					let b_len = u.int_in_range(0..=32)?;
					let bytes = u.bytes(b_len)?;
					v.push(bytes.to_vec());
				}
				Ok(Self::Binary(v))
			}
			6 => {
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
			_ => unreachable!(),
		}
	}

	fn to_picoarray(&self) -> Box<dyn Array> {
		match self {
			Self::Bool(v) => {
				let mut arr =
					ArrayBoolean::<NonNullable>::new();
				for &val in v {
					arr.push(val);
				}
				Box::new(arr)
			}
			Self::U8(v) => {
				let mut arr = ArrayU8::<NonNullable>::new();
				for &val in v {
					arr.push(val);
				}
				Box::new(arr)
			}
			Self::F32(v) => {
				let mut arr = ArrayF32::<NonNullable>::new();
				for &val in v {
					arr.push(val);
				}
				Box::new(arr)
			}
			Self::F64(v) => {
				let mut arr = ArrayF64::<NonNullable>::new();
				for &val in v {
					arr.push(val);
				}
				Box::new(arr)
			}
			Self::Utf8(v) => {
				let mut arr = ArrayUtf8::<NonNullable>::new();
				for s in v {
					arr.push(s);
				}
				Box::new(arr)
			}
			Self::Binary(v) => {
				let mut arr = ArrayBinary::<NonNullable>::new();
				for b in v {
					arr.push(b);
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
		match (self, other) {
			(Self::Bool(a), Self::Bool(b)) => a == b,
			(Self::U8(a), Self::U8(b)) => a == b,
			(Self::F32(a), Self::F32(b)) => {
				a.len() == b.len()
					&& a.iter().zip(b.iter()).all(
						|(x, y)| {
							x.to_bits()
								== y.to_bits()
						},
					)
			}
			(Self::F64(a), Self::F64(b)) => {
				a.len() == b.len()
					&& a.iter().zip(b.iter()).all(
						|(x, y)| {
							x.to_bits()
								== y.to_bits()
						},
					)
			}
			(Self::Utf8(a), Self::Utf8(b)) => a == b,
			(Self::Binary(a), Self::Binary(b)) => a == b,
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
	match original {
		AnyArray::Bool(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<BooleanArray>()
				.unwrap();
			AnyArray::Bool(
				(0..arr.len()).map(|i| arr.value(i)).collect(),
			)
		}
		AnyArray::U8(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<UInt8Array>()
				.unwrap();
			AnyArray::U8(
				(0..arr.len()).map(|i| arr.value(i)).collect(),
			)
		}
		AnyArray::F32(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<Float32Array>()
				.unwrap();
			AnyArray::F32(
				(0..arr.len()).map(|i| arr.value(i)).collect(),
			)
		}
		AnyArray::F64(_) => {
			let arr = col
				.as_any()
				.downcast_ref::<Float64Array>()
				.unwrap();
			AnyArray::F64(
				(0..arr.len()).map(|i| arr.value(i)).collect(),
			)
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
