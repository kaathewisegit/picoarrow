use picoarrow::{
	Error, Schema,
	array::{ArrayI32, ArrayI64, ArrayUtf8, NonNullable},
	ipc::{Compression, StreamWriter},
};

use std::io::Cursor;

fn make_schema() -> Schema {
	let a: ArrayI32<NonNullable> = ArrayI32::new();
	let b: ArrayUtf8<NonNullable> = ArrayUtf8::new();
	Schema::from_arrays([
		("a", &a as &dyn picoarrow::array::Array),
		("b", &b),
	])
}

#[test]
fn too_few_arrays() {
	let schema = make_schema();
	let buf = Cursor::new(Vec::new());
	let mut w = StreamWriter::new(buf, schema, Compression::None).unwrap();

	let a: ArrayI32<NonNullable> = ArrayI32::new();
	let err = w.write_batch([&a as &dyn picoarrow::array::Array]);
	assert!(matches!(
		err,
		Err(Error::BatchDifferentNumberOfArrays {
			expected: 2,
			got: 1,
		})
	));
}

#[test]
fn too_many_arrays() {
	let schema = make_schema();
	let buf = Cursor::new(Vec::new());
	let mut w = StreamWriter::new(buf, schema, Compression::None).unwrap();

	let a: ArrayI32<NonNullable> = ArrayI32::new();
	let b: ArrayUtf8<NonNullable> = ArrayUtf8::new();
	let c: ArrayI64<NonNullable> = ArrayI64::new();
	let err = w.write_batch([&a as &dyn picoarrow::array::Array, &b, &c]);
	assert!(matches!(
		err,
		Err(Error::BatchDifferentNumberOfArrays {
			expected: 2,
			got: 3,
		})
	));
}

#[test]
fn wrong_type() {
	let schema = make_schema();
	let buf = Cursor::new(Vec::new());
	let mut w = StreamWriter::new(buf, schema, Compression::None).unwrap();

	let a: ArrayI32<NonNullable> = ArrayI32::new();
	let wrong: ArrayI64<NonNullable> = ArrayI64::new();
	let err = w.write_batch([&a as &dyn picoarrow::array::Array, &wrong]);
	assert!(matches!(err, Err(Error::BatchSchemaMismatch(_))));
}
