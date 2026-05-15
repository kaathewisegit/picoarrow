use std::io::{Error as IoError, Write};

use flatbuffers::FlatBufferBuilder;

use super::{Compression, StreamWriter};
use crate::{
	array::Array,
	fb::{Block, Footer, FooterArgs, MetadataVersion},
};

pub struct FileWriter<W> {
	cursor: usize,
	writer: StreamWriter<W>,
	batches: Vec<Block>,
}

impl<W: Write> FileWriter<W> {
	pub fn new<'a>(
		mut writer: W,
		arrays: impl IntoIterator<Item = (&'a str, &'a dyn Array)>,
		compression: Compression,
	) -> Result<Self, IoError> {
		writer.write_all(b"ARROW1\0\0")?;
		let writer = StreamWriter::new(writer, arrays, compression)?;
		let cursor = 8 + writer.buf_metadata.len();

		Ok(Self {
			cursor,
			writer,
			batches: Vec::new(),
		})
	}

	pub fn write_batch<'a, I>(&mut self, arrays: I) -> Result<(), IoError>
	where
		I: IntoIterator<Item = &'a dyn Array>,
	{
		self.writer.write_batch(arrays)?;

		let metadata_len = self.writer.buf_metadata.len();
		let data_len = self.writer.buf_data.len();

		self.batches.push(Block::new(
			self.cursor as i64,
			metadata_len as i32,
			data_len as i64,
		));
		self.cursor += metadata_len + data_len;

		Ok(())
	}

	pub fn finish(self) -> Result<W, IoError> {
		let mut writer = self.writer.finish()?;

		let mut builder = FlatBufferBuilder::new();

		let batches = builder.create_vector(&self.batches);

		let footer = Footer::create(
			&mut builder,
			&FooterArgs {
				version: MetadataVersion::V5,
				schema: todo!(),
				dictionaries: None,
				recordBatches: Some(batches),
				custom_metadata: None,
			},
		);

		builder.finish(footer, None);
		let footer_bytes = builder.finished_data();
		writer.write_all(footer_bytes)?;
		let footer_len = footer_bytes.len() as i32;
		writer.write_all(&footer_len.to_le_bytes())?;

		writer.write_all(b"ARROW1")?;

		Ok(writer)
	}
}
