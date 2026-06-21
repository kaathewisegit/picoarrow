use std::io::Write;

use flatbuffers::FlatBufferBuilder;

use super::{Compression, StreamWriter};
use crate::{
	Result, Schema,
	array::Array,
	fb::{Block, Footer, FooterArgs, MetadataVersion},
};

/// A wrapper over [`StreamWriter`] which implements the Arrow IPC file format
pub struct FileWriter<W> {
	cursor: usize,
	writer: StreamWriter<W>,
	batches: Vec<Block>,
}

impl<W: Write> FileWriter<W> {
	/// Creates a new file IPC format writer
	///
	/// Attempts to write to the passed writer, see [`StreamWriter::new`]
	/// for details on that behavior.
	pub fn new(
		mut writer: W,
		schema: Schema,
		compression: Compression,
	) -> Result<Self> {
		writer.write_all(b"ARROW1\0\0")?;
		let writer = StreamWriter::new(writer, schema, compression)?;
		// 8 ARROW1 perifx
		// 4 continuation
		// 4 metadata legnth
		let cursor = 8 + 4 + 4 + writer.metadata_written;

		Ok(Self {
			cursor,
			writer,
			batches: Vec::new(),
		})
	}

	/// Write a record batch
	///
	/// See [`StreamWriter::write_batch`] for details.
	pub fn write_batch<'a, I>(&mut self, arrays: I) -> Result<()>
	where
		I: IntoIterator<Item = &'a dyn Array>,
	{
		self.writer.write_batch(arrays)?;

		// length + body
		let metadata_len = 4 + self.writer.metadata_written;
		let data_len = self.writer.buf_data.len();

		self.cursor += 4; // continuation
		self.batches.push(Block::new(
			self.cursor as i64,
			metadata_len as i32,
			data_len as i64,
		));
		self.cursor += metadata_len + data_len;

		Ok(())
	}

	/// Write the data footer and flush the writer
	pub fn finish(&mut self) -> Result<()> {
		self.writer.write_eos()?;

		let mut builder = FlatBufferBuilder::new();

		let schema = self.writer.schema.serialize(&mut builder);

		let batches = builder.create_vector(&self.batches);

		let footer = Footer::create(
			&mut builder,
			&FooterArgs {
				version: MetadataVersion::V5,
				schema: Some(schema),
				dictionaries: None,
				recordBatches: Some(batches),
				custom_metadata: None,
			},
		);

		let writer = &mut self.writer.writer;

		builder.finish(footer, None);
		let footer_bytes = builder.finished_data();
		writer.write_all(footer_bytes)?;
		let footer_len = footer_bytes.len() as i32;
		writer.write_all(&footer_len.to_le_bytes())?;

		writer.write_all(b"ARROW1")?;
		writer.flush()?;

		Ok(())
	}

	/// Returns the backing writer passed in `new`
	///
	/// This method doesn't close the stream.  `finish` must be called for
	/// that.
	pub fn into_inner(self) -> W {
		self.writer.writer
	}
}
