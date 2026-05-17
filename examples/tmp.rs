use picoarrow::{
	Schema,
	array::{
		Array, ArrayBoolean, ArrayF64, ArrayFixedSizeList, ArrayU8,
		NonNullable, Nullable,
	},
	ipc::{Compression, FileWriter},
};

use std::fs::File;

fn main() {
	let mut u = ArrayU8::<NonNullable>::new();

	let mut b = ArrayBoolean::<Nullable>::new();

	let nested = ArrayF64::<NonNullable>::new();
	let mut fs = ArrayFixedSizeList::<_, NonNullable>::new(nested, 4);

	#[cfg(not(feature = "zstd"))]
	let compression = Compression::None;
	#[cfg(feature = "zstd")]
	let compression = Compression::Zstd(3);

	let file = File::create("target/tmp.ipc.stream").unwrap();
	let mut writer = FileWriter::new(
		file,
		Schema::from_fields([
			u.make_field("primitive"),
			b.make_field("boolean+nullable"),
			fs.make_field("nested"),
		]),
		compression,
	)
	.unwrap();

	u.push(0x57);
	u.push(0x4f);
	u.push(0x57);

	b.push(true);
	b.push(false);
	b.push_null();

	fs.push(|n| {
		n.push(1.0);
		n.push(2.0);
		n.push(3.0);
		n.push(4.0);
	})
	.unwrap();
	fs.push(|n| {
		n.push(-1.0);
		n.push(-2.0);
		n.push(-3.0);
		n.push(-4.0);
	})
	.unwrap();
	fs.push(|n| {
		n.push(0.0);
		n.push(0.0);
		n.push(0.0);
		n.push(0.0);
	})
	.unwrap();

	writer.write_batch([u.as_dyn(), b.as_dyn(), fs.as_dyn()])
		.unwrap();
	writer.finish().unwrap();
}
