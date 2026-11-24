use std::io::Cursor;

use picoarrow::{
	Error, Schema,
	array::{
		Array, ArrayBinary, ArrayBoolean, ArrayF32, ArrayFixedBinary,
		ArrayI32, ArrayI64, ArrayPrimitive, ArrayU8, ArrayUtf8,
		NonNullable, Nullable,
	},
	ipc::{
		Column, Compression, FileReader, FileWriter, StreamReader,
		StreamWriter,
	},
};

fn write_file(
	schema: Schema,
	batches: &[Vec<&dyn Array>],
	compression: Compression,
) -> Vec<u8> {
	let mut writer =
		FileWriter::new(Vec::new(), schema, compression).unwrap();
	for batch in batches {
		writer.write_batch(batch.iter().copied()).unwrap();
	}
	writer.finish().unwrap();
	writer.into_inner()
}

#[test]
fn missing_schema() {
	assert!(matches!(
		StreamReader::new(Cursor::new(Vec::new())),
		Err(Error::MissingSchema)
	));
}

#[test]
fn invalid_continuation_marker() {
	assert!(matches!(
		StreamReader::new(Cursor::new([0; 4])),
		Err(Error::InvalidContinuationMarker)
	));
}

#[test]
fn truncated_schema() {
	assert!(matches!(
		StreamReader::new(Cursor::new([0xff; 4])),
		Err(Error::ReadFailed(_))
	));
}

#[test]
fn read_schema() {
	let mut ints = ArrayI32::<NonNullable>::new();
	ints.push(1);

	let mut strings = ArrayUtf8::<NonNullable>::new();
	strings.push("first").unwrap();

	let schema = Schema::from_arrays([
		("ints", &ints as &dyn Array),
		("strings", &strings),
	]);
	let writer = StreamWriter::new(
		Vec::new(),
		schema.clone(),
		Compression::None,
	)
	.unwrap();
	let data = writer.finish().unwrap();
	let reader = StreamReader::new(Cursor::new(data)).unwrap();

	assert_eq!(reader.schema(), &schema);
}

#[test]
fn read_non_nullable_batch() {
	let mut booleans = ArrayBoolean::<NonNullable>::new();
	booleans.push(true);
	booleans.push(false);

	let mut integers = ArrayI32::<NonNullable>::new();
	integers.push(-1);
	integers.push(2);

	let mut strings = ArrayUtf8::<NonNullable>::new();
	strings.push("").unwrap();
	strings.push("long string").unwrap();

	let mut binaries = ArrayBinary::<NonNullable>::new();
	binaries.push(b"").unwrap();
	binaries.push(&[0, 255, 1]).unwrap();

	let mut fixed_binaries = ArrayFixedBinary::<NonNullable>::new(2);
	fixed_binaries.push(b"ab").unwrap();
	fixed_binaries.push(&[255, 0]).unwrap();

	let schema = Schema::from_arrays([
		("booleans", &booleans as &dyn Array),
		("integers", &integers),
		("strings", &strings),
		("binaries", &binaries),
		("fixed_binaries", &fixed_binaries),
	]);
	let arrays: Vec<&dyn Array> = vec![
		&booleans,
		&integers,
		&strings,
		&binaries,
		&fixed_binaries,
	];

	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::None)
			.unwrap();
	writer.write_batch(arrays).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	let columns = reader.next().unwrap().unwrap();
	assert_eq!(
		columns,
		vec![
			Column::BooleanNonNullable(booleans),
			Column::I32NonNullable(integers),
			Column::Utf8NonNullable(strings),
			Column::BinaryNonNullable(binaries),
			Column::FixedBinaryNonNullable(fixed_binaries),
		]
	);
	assert!(reader.next().is_none());
}

#[test]
fn read_nullable_batch() {
	let mut booleans = ArrayBoolean::<Nullable>::new();
	booleans.push(Some(true));
	booleans.push_null();
	booleans.push(Some(false));

	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(-1));
	integers.push_null();
	integers.push(Some(2));

	let mut strings = ArrayUtf8::<Nullable>::new();
	strings.push(Some("first")).unwrap();
	strings.push_null();
	strings.push(Some("third")).unwrap();

	let mut binaries = ArrayBinary::<Nullable>::new();
	binaries.push(Some(b"first")).unwrap();
	binaries.push_null();
	binaries.push(Some(&[0, 255, 1])).unwrap();

	let mut fixed_binaries = ArrayFixedBinary::<Nullable>::new(2);
	fixed_binaries.push_some(b"ab").unwrap();
	fixed_binaries.push_null();
	fixed_binaries.push_some(&[255, 0]).unwrap();

	let schema = Schema::from_arrays([
		("booleans", &booleans as &dyn Array),
		("integers", &integers),
		("strings", &strings),
		("binaries", &binaries),
		("fixed_binaries", &fixed_binaries),
	]);
	let arrays: Vec<&dyn Array> = vec![
		&booleans,
		&integers,
		&strings,
		&binaries,
		&fixed_binaries,
	];

	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::None)
			.unwrap();
	writer.write_batch(arrays).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	let columns = reader.next().unwrap().unwrap();
	assert_eq!(
		columns,
		vec![
			Column::Boolean(booleans),
			Column::I32(integers),
			Column::Utf8(strings),
			Column::Binary(binaries),
			Column::FixedBinary(fixed_binaries),
		]
	);
	assert!(reader.next().is_none());
}

#[test]
fn read_primitive_types() {
	macro_rules! primitive {
		($name:ident, $ty:ty, $values:expr) => {{
			let mut values =
				ArrayPrimitive::<$ty, NonNullable>::new();
			for value in $values {
				values.push(value);
			}
			values
		}};
	}

	let u8s = primitive!(u8s, u8, [0, 255]);
	let u16s = primitive!(u16s, u16, [1, 65_535]);
	let u32s = primitive!(u32s, u32, [2, u32::MAX]);
	let u64s = primitive!(u64s, u64, [3, u64::MAX]);
	let i8s = primitive!(i8s, i8, [-128, 127]);
	let i16s = primitive!(i16s, i16, [-32_768, 32_767]);
	let i32s = primitive!(i32s, i32, [i32::MIN, i32::MAX]);
	let i64s = primitive!(i64s, i64, [i64::MIN, i64::MAX]);
	let f32s = primitive!(f32s, f32, [-1.5, 2.5]);
	let f64s = primitive!(f64s, f64, [-3.5, 4.5]);

	let schema = Schema::from_arrays([
		("u8s", &u8s as &dyn Array),
		("u16s", &u16s),
		("u32s", &u32s),
		("u64s", &u64s),
		("i8s", &i8s),
		("i16s", &i16s),
		("i32s", &i32s),
		("i64s", &i64s),
		("f32s", &f32s),
		("f64s", &f64s),
	]);
	let arrays: Vec<&dyn Array> = vec![
		&u8s, &u16s, &u32s, &u64s, &i8s, &i16s, &i32s, &i64s, &f32s,
		&f64s,
	];

	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::None)
			.unwrap();
	writer.write_batch(arrays).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![
			Column::U8NonNullable(u8s),
			Column::U16NonNullable(u16s),
			Column::U32NonNullable(u32s),
			Column::U64NonNullable(u64s),
			Column::I8NonNullable(i8s),
			Column::I16NonNullable(i16s),
			Column::I32NonNullable(i32s),
			Column::I64NonNullable(i64s),
			Column::F32NonNullable(f32s),
			Column::F64NonNullable(f64s),
		]
	);
}

#[test]
#[cfg(feature = "half")]
fn read_f16() {
	let mut halves = ArrayPrimitive::<half::f16, NonNullable>::new();
	halves.push(half::f16::from_f32(-1.5));
	halves.push(half::f16::from_f32(2.5));

	let schema = Schema::from_arrays([("halves", &halves as &dyn Array)]);
	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::None)
			.unwrap();
	writer.write_batch([&halves as &dyn Array]).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::F16NonNullable(halves)]
	);
}

#[test]
fn read_multiple_batches() {
	let mut first = ArrayI32::<NonNullable>::new();
	first.push(1);

	let mut second = ArrayI32::<NonNullable>::new();
	second.push(2);
	second.push(3);

	let schema = Schema::from_arrays([("values", &first as &dyn Array)]);
	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::None)
			.unwrap();
	writer.write_batch([&first as &dyn Array]).unwrap();
	writer.write_batch([&second as &dyn Array]).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32NonNullable(first)]
	);
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32NonNullable(second)]
	);
	assert!(reader.next().is_none());
	assert!(reader.next().is_none());
}

#[test]
#[cfg(feature = "lz4")]
fn read_lz4_batch() {
	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(1));
	integers.push_null();

	let schema =
		Schema::from_arrays([("integers", &integers as &dyn Array)]);
	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::LZ4)
			.unwrap();
	writer.write_batch([&integers as &dyn Array]).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32(integers)]
	);
}

#[test]
#[cfg(feature = "zstd")]
fn read_zstd_batch() {
	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(1));
	integers.push_null();

	let schema =
		Schema::from_arrays([("integers", &integers as &dyn Array)]);
	let mut writer =
		StreamWriter::new(Vec::new(), schema, Compression::Zstd(0))
			.unwrap();
	writer.write_batch([&integers as &dyn Array]).unwrap();
	let data = writer.finish().unwrap();

	let mut reader = StreamReader::new(Cursor::new(data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32(integers)]
	);
}
#[test]
fn roundtrip_non_nullable_batch() {
	let mut booleans = ArrayBoolean::<NonNullable>::new();
	booleans.push(true);
	booleans.push(false);

	let mut integers = ArrayI32::<NonNullable>::new();
	integers.push(-1);
	integers.push(2);

	let mut longs = ArrayI64::<NonNullable>::new();
	longs.push(i64::MIN);
	longs.push(i64::MAX);

	let mut floats = ArrayF32::<NonNullable>::new();
	floats.push(0.5);
	floats.push(-1.25);

	let mut bytes = ArrayU8::<NonNullable>::new();
	bytes.push(0);
	bytes.push(255);

	let mut strings = ArrayUtf8::<NonNullable>::new();
	strings.push("").unwrap();
	strings.push("long string").unwrap();

	let mut binaries = ArrayBinary::<NonNullable>::new();
	binaries.push(b"").unwrap();
	binaries.push(&[0, 255, 1]).unwrap();

	let mut fixed_binaries = ArrayFixedBinary::<NonNullable>::new(2);
	fixed_binaries.push(b"ab").unwrap();
	fixed_binaries.push(&[255, 0]).unwrap();

	let schema = Schema::from_arrays([
		("booleans", &booleans as &dyn Array),
		("integers", &integers),
		("longs", &longs),
		("floats", &floats),
		("bytes", &bytes),
		("strings", &strings),
		("binaries", &binaries),
		("fixed_binaries", &fixed_binaries),
	]);

	let data = write_file(
		schema.clone(),
		&[vec![
			&booleans,
			&integers,
			&longs,
			&floats,
			&bytes,
			&strings,
			&binaries,
			&fixed_binaries,
		]],
		Compression::None,
	);

	let mut reader = FileReader::new(Cursor::new(&data)).unwrap();
	assert_eq!(reader.schema(), &schema);
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![
			Column::BooleanNonNullable(booleans),
			Column::I32NonNullable(integers),
			Column::I64NonNullable(longs),
			Column::F32NonNullable(floats),
			Column::U8NonNullable(bytes),
			Column::Utf8NonNullable(strings),
			Column::BinaryNonNullable(binaries),
			Column::FixedBinaryNonNullable(fixed_binaries),
		]
	);
	assert!(reader.next().is_none());
}

#[test]
fn roundtrip_nullable_batch() {
	let mut booleans = ArrayBoolean::<Nullable>::new();
	booleans.push(Some(true));
	booleans.push_null();

	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(1));
	integers.push_null();

	let mut strings = ArrayUtf8::<Nullable>::new();
	strings.push(Some("first")).unwrap();
	strings.push_null();

	let mut fixed_binaries = ArrayFixedBinary::<Nullable>::new(2);
	fixed_binaries.push(Some(&[0, 1])).unwrap();
	fixed_binaries.push_null();

	let schema = Schema::from_arrays([
		("booleans", &booleans as &dyn Array),
		("integers", &integers),
		("strings", &strings),
		("fixed_binaries", &fixed_binaries),
	]);

	let data = write_file(
		schema.clone(),
		&[vec![&booleans, &integers, &strings, &fixed_binaries]],
		Compression::None,
	);

	let mut reader = FileReader::new(Cursor::new(&data)).unwrap();
	assert_eq!(reader.schema(), &schema);
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![
			Column::Boolean(booleans),
			Column::I32(integers),
			Column::Utf8(strings),
			Column::FixedBinary(fixed_binaries),
		]
	);
	assert!(reader.next().is_none());
}

#[test]
fn roundtrip_multiple_batches() {
	let mut first = ArrayI32::<NonNullable>::new();
	first.push(1);

	let mut second = ArrayI32::<NonNullable>::new();
	second.push(2);
	second.push(3);

	let schema = Schema::from_arrays([("values", &first as &dyn Array)]);
	let data = write_file(
		schema,
		&[vec![&first as &dyn Array], vec![&second]],
		Compression::None,
	);

	let mut reader = FileReader::new(Cursor::new(&data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32NonNullable(first)]
	);
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32NonNullable(second)]
	);
	assert!(reader.next().is_none());
}

#[test]
#[cfg(feature = "lz4")]
fn roundtrip_lz4_batch() {
	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(1));
	integers.push_null();

	let schema =
		Schema::from_arrays([("integers", &integers as &dyn Array)]);
	let data = write_file(schema, &[vec![&integers]], Compression::LZ4);

	let mut reader = FileReader::new(Cursor::new(&data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32(integers)]
	);
	assert!(reader.next().is_none());
}

#[test]
#[cfg(feature = "zstd")]
fn roundtrip_zstd_batch() {
	let mut integers = ArrayI32::<Nullable>::new();
	integers.push(Some(1));
	integers.push_null();

	let schema =
		Schema::from_arrays([("integers", &integers as &dyn Array)]);
	let data = write_file(schema, &[vec![&integers]], Compression::Zstd(0));

	let mut reader = FileReader::new(Cursor::new(&data)).unwrap();
	assert_eq!(
		reader.next().unwrap().unwrap(),
		vec![Column::I32(integers)]
	);
	assert!(reader.next().is_none());
}

#[test]
fn empty_file() {
	assert!(matches!(
		FileReader::new(Cursor::new(Vec::new())),
		Err(Error::InvalidFileMagic)
	));
}
