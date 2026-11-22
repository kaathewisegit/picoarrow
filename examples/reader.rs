use std::{fs::File, io::BufReader};

use picoarrow::ipc::StreamReader;

fn main() {
	let path = std::env::args()
		.nth(1)
		.expect("expected an Arrow stream file");

	let file = File::open(path).expect("failed to open stream file");
	let mut reader = StreamReader::new(BufReader::new(file))
		.expect("failed to read schema");

	println!("{:#?}", reader.schema());

	for batch in &mut reader {
		let batch = batch.expect("failed to read record batch");
		println!("{batch:#?}");
	}
}
