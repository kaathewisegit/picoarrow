use picoarrow::{
	Error, Schema,
	array::{
		Array, ArrayBoolean, ArrayI32, ArrayI64, ArrayUtf8, NonNullable,
	},
	ipc::{Compression, StreamWriter},
};

use arrow_ipc::reader::StreamReader as ArrowStreamReader;
use arrow_schema::Schema as ArrowSchema;

use std::collections::HashMap;
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

#[test]
fn different_lengths_second_array() {
	let schema = make_schema();
	let buf = Cursor::new(Vec::new());
	let mut w = StreamWriter::new(buf, schema, Compression::None).unwrap();

	let mut a: ArrayI32<NonNullable> = ArrayI32::new();
	a.push(1);
	a.push(2);

	let mut b: ArrayUtf8<NonNullable> = ArrayUtf8::new();
	b.push("x").unwrap();

	let err = w.write_batch([&a as &dyn picoarrow::array::Array, &b]);
	assert!(matches!(
		err,
		Err(Error::BatchDifferentLengths(inner)) if inner.0 == 2 && inner.1 == 1
	));
}

fn roundtrip_with_batch(schema: Schema, arrays: &[&dyn Array]) -> ArrowSchema {
	let mut w = StreamWriter::new(Vec::new(), schema, Compression::None)
		.unwrap();
	w.write_batch(arrays.iter().copied()).unwrap();
	let data = w.finish().unwrap();
	let mut bytes = &data[..];
	let reader = ArrowStreamReader::try_new(&mut bytes, None).unwrap();
	reader.schema().as_ref().clone()
}

#[test]
fn field_metadata_after_empty_preceding_fields() {
	let mut a = ArrayI32::<NonNullable>::new();
	a.push(1);
	let mut b = ArrayUtf8::<NonNullable>::new();
	b.push("x").unwrap();
	let mut c = ArrayI64::<NonNullable>::new();
	c.push(2);
	let mut d = ArrayBoolean::<NonNullable>::new();
	d.push(true);

	let arrays: Vec<&dyn Array> = vec![&a, &b, &c, &d];
	let mut schema = Schema::from_arrays([
		("a", &a as &dyn Array),
		("b", &b),
		("c", &c),
		("d", &d),
	]);
	schema.fields[3].custom_metadata =
		vec![("key".to_string(), "value".to_string())];

	let arrow_schema = roundtrip_with_batch(schema, &arrays);
	let fields = arrow_schema.fields();
	assert_eq!(fields.len(), 4);
	for i in 0..3 {
		assert!(
			fields[i].metadata().is_empty(),
			"field {i} should have no metadata"
		);
	}
	let expected: HashMap<String, String> =
		[("key".to_string(), "value".to_string())]
			.into_iter()
			.collect();
	assert_eq!(fields[3].metadata(), &expected);
}

#[test]
fn field_metadata_with_empty_fields_before_and_after() {
	let mut a = ArrayI32::<NonNullable>::new();
	a.push(1);
	let mut b = ArrayUtf8::<NonNullable>::new();
	b.push("x").unwrap();
	let mut c = ArrayI64::<NonNullable>::new();
	c.push(2);
	let mut d = ArrayBoolean::<NonNullable>::new();
	d.push(true);

	let arrays: Vec<&dyn Array> = vec![&a, &b, &c, &d];
	let mut schema = Schema::from_arrays([
		("a", &a as &dyn Array),
		("b", &b),
		("c", &c),
		("d", &d),
	]);
	schema.fields[1].custom_metadata =
		vec![("b_key".to_string(), "b_value".to_string())];

	let arrow_schema = roundtrip_with_batch(schema, &arrays);
	let fields = arrow_schema.fields();
	assert_eq!(fields.len(), 4);
	for i in [0, 2, 3] {
		assert!(
			fields[i].metadata().is_empty(),
			"field {i} should have no metadata"
		);
	}
	let expected: HashMap<String, String> =
		[("b_key".to_string(), "b_value".to_string())]
			.into_iter()
			.collect();
	assert_eq!(fields[1].metadata(), &expected);
}
