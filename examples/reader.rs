use std::process::ExitCode;

use picoarrow::ipc::StreamReader;

fn main() -> ExitCode {
	let Some(path) = std::env::args().nth(1) else {
		eprintln!("usage: reader <arrow-stream-file>");
		return ExitCode::from(2);
	};

	let file = match std::fs::File::open(&path) {
		Ok(file) => file,
		Err(e) => {
			eprintln!("failed to open {path}: {e}");
			return ExitCode::FAILURE;
		}
	};

	let mut reader = match StreamReader::new(std::io::BufReader::new(file))
	{
		Ok(reader) => reader,
		Err(e) => {
			eprintln!("failed to read schema: {e}");
			return ExitCode::FAILURE;
		}
	};

	println!("{:#?}", reader.schema());

	let result = (|| {
		for batch in &mut reader {
			batch?;
		}
		Ok::<_, picoarrow::Error>(())
	})();

	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("failed to read stream: {e}");
			ExitCode::FAILURE
		}
	}
}
