use arbitrary::{Arbitrary, Result, Unstructured};
use arbtest::arbtest;
use arrow_ipc::reader::{
	FileReader as ArrowFileReader, StreamReader as ArrowStreamReader,
};

use std::{collections::HashMap, io::Cursor};

use picoarrow::{
	Schema,
	ipc::{Compression, FileWriter, StreamWriter},
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Metadata(Vec<(String, String)>);

impl<'a> Arbitrary<'a> for Metadata {
	fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
		Ok(Metadata(u.arbitrary_iter()?.collect::<Result<_, _>>()?))
	}
}

fn check_metadata_roundtrip(
	metadata: &Metadata,
	compression: Compression,
	format: IpcFormat,
) {
	let pico_schema = Schema {
		fields: vec![],
		custom_metadata: metadata.0.clone(),
	};

	let data = match format {
		IpcFormat::Stream => {
			let writer = StreamWriter::new(
				Vec::new(),
				pico_schema,
				compression,
			)
			.unwrap();
			writer.finish().unwrap()
		}
		IpcFormat::File => {
			let mut writer = FileWriter::new(
				Vec::new(),
				pico_schema,
				compression,
			)
			.unwrap();
			writer.finish().unwrap();
			writer.into_inner()
		}
	};

	let arrow_schema = match format {
		IpcFormat::Stream => {
			let mut reader = &data[..];
			let reader =
				ArrowStreamReader::try_new(&mut reader, None)
					.unwrap();
			reader.schema().clone()
		}
		IpcFormat::File => {
			let reader = ArrowFileReader::try_new(
				Cursor::new(data),
				None,
			)
			.unwrap();
			reader.schema().clone()
		}
	};

	let expected: HashMap<String, String> =
		metadata.0.iter().cloned().collect();
	assert_eq!(
		arrow_schema.metadata(),
		&expected,
		"metadata mismatch\nexpected: {expected:?}\ngot: {:?}",
		arrow_schema.metadata()
	);
}

enum IpcFormat {
	Stream,
	File,
}

#[test]
fn metadata_roundtrip_stream() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::None,
			IpcFormat::Stream,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
fn metadata_roundtrip_file() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::None,
			IpcFormat::File,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
#[cfg(feature = "lz4")]
fn metadata_roundtrip_stream_lz4() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::LZ4,
			IpcFormat::Stream,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
#[cfg(feature = "lz4")]
fn metadata_roundtrip_file_lz4() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::LZ4,
			IpcFormat::File,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
#[cfg(feature = "zstd")]
fn metadata_roundtrip_stream_zstd() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::Zstd(0),
			IpcFormat::Stream,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
#[cfg(feature = "zstd")]
fn metadata_roundtrip_file_zstd() {
	arbtest(|u| {
		let metadata = Metadata::arbitrary(u)?;
		check_metadata_roundtrip(
			&metadata,
			Compression::Zstd(0),
			IpcFormat::File,
		);
		Ok(())
	})
	.size_min(2u32.pow(16));
}
