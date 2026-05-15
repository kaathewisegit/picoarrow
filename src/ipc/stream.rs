use std::{
	io::{Error as IoError, Write},
	mem::take,
};

use super::{Compression, write_batch, write_schema};
use crate::array::Array;

pub struct StreamWriter<W> {
	buf_metadata: Vec<u8>,
	buf_data: Vec<u8>,
	compression: Compression,
	writer: W,
}

fn write_continuation<W: Write>(w: &mut W) -> Result<(), IoError> {
	w.write_all(&[0xFF, 0xFF, 0xFF, 0xFF])
}

fn write_metadata<W: Write>(w: &mut W, metadata: &[u8]) -> Result<(), IoError> {
	assert_eq!(metadata.len() % 8, 0);
	let length = metadata.len() as i32;
	w.write_all(&length.to_le_bytes())?;
	w.write_all(metadata)
}

impl<W: Write> StreamWriter<W> {
	pub fn new<'a>(
		mut writer: W,
		arrays: impl IntoIterator<Item = (&'a str, &'a dyn Array)>,
		compression: Compression,
	) -> Result<Self, IoError> {
		let buf_metadata = Vec::new();

		let builder = write_schema(buf_metadata, arrays);
		let schema_data = builder.finished_data();
		write_continuation(&mut writer)?;
		write_metadata(&mut writer, schema_data)?;

		let (mut buf_metadata, _) = builder.collapse();
		buf_metadata.clear();

		Ok(Self {
			buf_metadata,
			buf_data: Vec::new(),
			compression,
			writer,
		})
	}

	pub fn write_batch<'a, I>(&mut self, arrays: I) -> Result<(), IoError>
	where
		I: IntoIterator<Item = &'a dyn Array>,
	{
		let (builder, mut buf_data) = write_batch(
			take(&mut self.buf_metadata),
			take(&mut self.buf_data),
			arrays,
			self.compression,
		);

		write_continuation(&mut self.writer)?;
		write_metadata(&mut self.writer, builder.finished_data())?;
		self.writer.write_all(&buf_data)?;

		let mut buf_metadata = builder.collapse().0;
		buf_metadata.clear();
		buf_data.clear();

		self.buf_metadata = buf_metadata;
		self.buf_data = buf_data;

		Ok(())
	}
}
