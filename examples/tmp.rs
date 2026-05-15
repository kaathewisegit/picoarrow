use microarrow::{
	array::{Array, ArrayF64, ArrayFixedSizeList, ArrayU32, NonNullable},
	ipc::{Compression, RecordBatchBuilder, SchemaBuilder},
};

use std::{fs::File, io::Write};

fn main() {
	let mut f = File::create("tmp.ipc.stream").unwrap();

	let mut u = ArrayU32::<NonNullable>::new();
	u.push(0);
	u.push(1);
	u.push(2);

	let nested = ArrayF64::<NonNullable>::new();
	let mut fs = ArrayFixedSizeList::<_, NonNullable>::new(nested, 4);
	fs.push(|n| {
		n.push(1.0);
		n.push(2.0);
		n.push(3.0);
		n.push(4.0);
	});
	fs.push(|n| {
		n.push(-1.0);
		n.push(-2.0);
		n.push(-3.0);
		n.push(-4.0);
	});
	fs.push(|n| {
		n.push(0.0);
		n.push(0.0);
		n.push(0.0);
		n.push(0.0);
	});

	let mut schema = SchemaBuilder::new();
	schema.add_array("u32", &u);
	schema.add_array("fixed size", &fs);

	f.write_all(&0xFFFFFFFFu32.to_le_bytes()).unwrap();
	let schema_bytes = schema.finish();
	let schema_len = schema_bytes.len() as i32;
	f.write_all(&schema_len.to_le_bytes()).unwrap();
	f.write_all(schema_bytes).unwrap();

	let mut builder = RecordBatchBuilder::new(u.len(), Compression::None);
	builder.add_array(&u);
	builder.add_array(&fs);

	f.write_all(&0xFFFFFFFFu32.to_le_bytes()).unwrap();
	let builder_bytes = builder.finish();
	let builder_len = builder_bytes.len() as i32;
	f.write_all(&builder_len.to_le_bytes()).unwrap();
	f.write_all(builder_bytes).unwrap();
	f.write_all(builder.data()).unwrap();
}
