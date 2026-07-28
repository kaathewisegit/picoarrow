#[cfg(feature = "half")]
use picoarrow::array::ArrayF16;
use picoarrow::{
	Schema,
	array::{
		Array, ArrayBinary, ArrayBoolean, ArrayF32, ArrayF64,
		ArrayFixedBinary, ArrayI8, ArrayI16, ArrayI32, ArrayI64,
		ArrayU8, ArrayU16, ArrayU32, ArrayU64, ArrayUtf8, NonNullable,
		Nullable,
	},
	ipc::{Compression, StreamWriter},
};

use arrow_ipc::reader::StreamReader as ArrowStreamReader;
use arrow_schema::{DataType as ArrowDataType, Schema as ArrowSchema};

use std::collections::HashMap;

/// Serializes a `Schema` via the IPC stream format and reads it back
/// using the reference `arrow-ipc` reader.
fn roundtrip_schema(schema: Schema) -> ArrowSchema {
	let writer = StreamWriter::new(Vec::new(), schema, Compression::None)
		.unwrap();
	let data = writer.finish().unwrap();
	let mut bytes = &data[..];
	let reader = ArrowStreamReader::try_new(&mut bytes, None).unwrap();
	reader.schema().as_ref().clone()
}

#[test]
fn empty_schema_roundtrip() {
	let schema = Schema {
		fields: vec![],
		custom_metadata: vec![],
	};
	let arrow_schema = roundtrip_schema(schema);
	assert!(arrow_schema.fields().is_empty());
	assert!(arrow_schema.metadata().is_empty());
}

#[test]
fn schema_metadata_roundtrip() {
	let schema = Schema {
		fields: vec![],
		custom_metadata: vec![
			("key1".to_string(), "value1".to_string()),
			("key2".to_string(), "value2".to_string()),
		],
	};
	let arrow_schema = roundtrip_schema(schema);

	let expected: HashMap<String, String> = [
		("key1".to_string(), "value1".to_string()),
		("key2".to_string(), "value2".to_string()),
	]
	.into_iter()
	.collect();
	assert_eq!(arrow_schema.metadata(), &expected);
}

#[test]
fn field_types_roundtrip() {
	// (name, array, expected arrow type, nullable)
	let base: Vec<(&str, Box<dyn Array>, ArrowDataType, bool)> = vec![
		(
			"bool",
			Box::new(ArrayBoolean::<NonNullable>::new()),
			ArrowDataType::Boolean,
			false,
		),
		(
			"i8",
			Box::new(ArrayI8::<NonNullable>::new()),
			ArrowDataType::Int8,
			false,
		),
		(
			"i16",
			Box::new(ArrayI16::<NonNullable>::new()),
			ArrowDataType::Int16,
			false,
		),
		(
			"i32",
			Box::new(ArrayI32::<NonNullable>::new()),
			ArrowDataType::Int32,
			false,
		),
		(
			"i64",
			Box::new(ArrayI64::<NonNullable>::new()),
			ArrowDataType::Int64,
			false,
		),
		(
			"u8",
			Box::new(ArrayU8::<NonNullable>::new()),
			ArrowDataType::UInt8,
			false,
		),
		(
			"u16",
			Box::new(ArrayU16::<NonNullable>::new()),
			ArrowDataType::UInt16,
			false,
		),
		(
			"u32",
			Box::new(ArrayU32::<NonNullable>::new()),
			ArrowDataType::UInt32,
			false,
		),
		(
			"u64",
			Box::new(ArrayU64::<NonNullable>::new()),
			ArrowDataType::UInt64,
			false,
		),
		(
			"f32",
			Box::new(ArrayF32::<NonNullable>::new()),
			ArrowDataType::Float32,
			false,
		),
		(
			"f64",
			Box::new(ArrayF64::<NonNullable>::new()),
			ArrowDataType::Float64,
			false,
		),
		(
			"utf8",
			Box::new(ArrayUtf8::<NonNullable>::new()),
			ArrowDataType::Utf8,
			false,
		),
		(
			"binary",
			Box::new(ArrayBinary::<NonNullable>::new()),
			ArrowDataType::Binary,
			false,
		),
		(
			"fsb",
			Box::new(ArrayFixedBinary::<NonNullable>::new(16)),
			ArrowDataType::FixedSizeBinary(16),
			false,
		),
		(
			"nullable_i32",
			Box::new(ArrayI32::<Nullable>::new()),
			ArrowDataType::Int32,
			true,
		),
		(
			"nullable_bool",
			Box::new(ArrayBoolean::<Nullable>::new()),
			ArrowDataType::Boolean,
			true,
		),
	];
	#[cfg(feature = "half")]
	let extra: Vec<(&str, Box<dyn Array>, ArrowDataType, bool)> = vec![(
		"f16",
		Box::new(ArrayF16::<NonNullable>::new()),
		ArrowDataType::Float16,
		false,
	)];
	#[cfg(not(feature = "half"))]
	let extra: Vec<(&str, Box<dyn Array>, ArrowDataType, bool)> = vec![];

	let entries: Vec<(&str, Box<dyn Array>, ArrowDataType, bool)> =
		base.into_iter().chain(extra).collect();

	let pairs: Vec<(&str, &dyn Array)> = entries
		.iter()
		.map(|(name, array, _, _)| (*name, &**array))
		.collect();
	let schema = Schema::from_arrays(pairs);

	let arrow_schema = roundtrip_schema(schema);
	let fields = arrow_schema.fields();

	assert_eq!(fields.len(), entries.len());
	for (i, (name, _, ty, nullable)) in entries.iter().enumerate() {
		let field = &fields[i];
		assert_eq!(field.name(), *name, "field {i} name");
		assert_eq!(field.data_type(), ty, "field {i} data type");
		assert_eq!(
			field.is_nullable(),
			*nullable,
			"field {i} nullable"
		);
		assert!(field.metadata().is_empty(), "field {i} metadata");
	}
}

#[test]
fn field_metadata_roundtrip() {
	let a = ArrayI32::<NonNullable>::new();
	let b = ArrayUtf8::<NonNullable>::new();
	let pairs: Vec<(&str, &dyn Array)> = vec![("a", &a), ("b", &b)];
	let mut schema = Schema::from_arrays(pairs);

	schema.fields[0].custom_metadata = vec![
		("a_key".to_string(), "a_value".to_string()),
		("shared".to_string(), "first".to_string()),
	];
	schema.fields[1].custom_metadata =
		vec![("b_key".to_string(), "b_value".to_string())];

	let arrow_schema = roundtrip_schema(schema);
	let fields = arrow_schema.fields();
	assert_eq!(fields.len(), 2);

	let expected_a: HashMap<String, String> = [
		("a_key".to_string(), "a_value".to_string()),
		("shared".to_string(), "first".to_string()),
	]
	.into_iter()
	.collect();
	assert_eq!(fields[0].metadata(), &expected_a);

	let expected_b: HashMap<String, String> =
		[("b_key".to_string(), "b_value".to_string())]
			.into_iter()
			.collect();
	assert_eq!(fields[1].metadata(), &expected_b);
}
