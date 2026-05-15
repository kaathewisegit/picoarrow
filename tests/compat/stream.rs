use arrow_array::RecordBatch;
use arrow_ipc::reader::StreamReader;

use picoarrow::{
	Schema,
	array::{Array, ArrayF64, NonNullable},
	ipc::{Compression, StreamWriter},
};

fn read_ipc_stream(mut buffer: &[u8]) -> Vec<RecordBatch> {
	let reader = StreamReader::try_new(&mut buffer, None).unwrap();

	reader.map(|b| b.unwrap()).collect()
}

#[test]
fn primitive_float() {
	let mut array = ArrayF64::<NonNullable>::new();
	array.push(0.0);
	array.push(1.0);
	array.push(-1.0);

	let buffer = Vec::<u8>::new();
	let mut writer = StreamWriter::new(
		buffer,
		Schema::from_fields([array.make_field("array")]),
		Compression::None,
	)
	.unwrap();
	writer.write_batch([&array as &dyn Array]).unwrap();

	let buffer = writer.finish().unwrap();

	let batches = read_ipc_stream(&buffer);
	let batch = batches.first().unwrap();

	let column = batch.column(0);
	let float_array = column
		.as_any()
		.downcast_ref::<arrow_array::Float64Array>()
		.to_owned()
		.unwrap();

	assert_eq!(float_array.len(), 3);
	assert_eq!(float_array.value(0), 0.0);
	assert_eq!(float_array.value(1), 1.0);
	assert_eq!(float_array.value(2), -1.0);
}
