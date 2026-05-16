use flatbuffers::FlatBufferBuilder;
#[cfg(feature = "zstd")]
use zstd::stream::Encoder;

use std::{
	io::{Error as IoError, Write},
	mem::take,
};

use super::Compression;
#[cfg(any(feature = "zstd", feature = "lz4"))]
use crate::fb::{
	BodyCompression, BodyCompressionArgs, BodyCompressionMethod,
	CompressionType,
};
use crate::{
	Error, Result,
	array::Array,
	fb::{
		Buffer, FieldNode, Message, MessageArgs, MessageHeader,
		MetadataVersion, RecordBatch, RecordBatchArgs,
	},
	schema::Schema,
};

/// An IPC writer for [the streaming format][s]
///
/// The format consists of a header in form of a schema and a number of record
/// batches.  `StreamWriter` currently doesn't support dictionaries.
///
/// [s]: https://arrow.apache.org/docs/format/Columnar.html#ipc-streaming-format
pub struct StreamWriter<W> {
	pub(crate) buf_metadata: Vec<u8>,
	pub(crate) buf_data: Vec<u8>,
	pub(crate) schema: Schema,
	pub(crate) compression: Compression,
	pub(crate) metadata_written: usize,
	pub(crate) writer: W,
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
	/// Creates a new writer
	///
	/// This method will attempt to write the schema to the passed writer
	/// and returns [`Error::WriteFailed`] on failure.
	pub fn new(
		mut writer: W,
		schema: Schema,
		compression: Compression,
	) -> Result<Self> {
		write_continuation(&mut writer)?;

		let buf_metadata = Vec::new();

		let builder = write_schema(buf_metadata, &schema);

		let schema_data = builder.finished_data();
		let metadata_written = schema_data.len();
		write_metadata(&mut writer, schema_data)?;

		let (buf_metadata, _) = builder.collapse();

		Ok(Self {
			buf_metadata,
			buf_data: Vec::new(),
			schema,
			compression,
			metadata_written,
			writer,
		})
	}

	/// Write a set of arrays
	///
	/// They must be passed in exactly the same order they were in the
	/// schema.
	// TODO: schema verification
	pub fn write_batch<'a, I>(&mut self, arrays: I) -> Result<()>
	where
		I: IntoIterator<Item = &'a dyn Array>,
	{
		self.buf_metadata.clear();
		self.buf_data.clear();

		let (builder, buf_data) = write_batch(
			take(&mut self.buf_metadata),
			take(&mut self.buf_data),
			arrays,
			self.compression,
		)?;

		write_continuation(&mut self.writer)?;

		let metadata_bytes = builder.finished_data();
		self.metadata_written = metadata_bytes.len();
		write_metadata(&mut self.writer, metadata_bytes)?;

		self.writer.write_all(&buf_data)?;

		self.buf_metadata = builder.collapse().0;
		self.buf_data = buf_data;

		Ok(())
	}

	pub(crate) fn write_eos(&mut self) -> Result<()> {
		self.writer
			.write_all(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0])?;
		Ok(())
	}

	/// Flushes the destination writer and returns it
	pub fn finish(mut self) -> Result<W, IoError> {
		self.writer.flush()?;
		Ok(self.writer)
	}
}

fn write_schema<'fbb>(
	buf_metadata: Vec<u8>,
	schema: &Schema,
) -> FlatBufferBuilder<'fbb> {
	let mut builder = FlatBufferBuilder::from_vec(buf_metadata);

	let schema = schema.serialize(&mut builder).as_union_value();

	let message = Message::create(
		&mut builder,
		&MessageArgs {
			version: MetadataVersion::V5,
			header: Some(schema),
			header_type: MessageHeader::Schema,
			bodyLength: 0,
			custom_metadata: None,
		},
	);

	builder.finish(message, None);

	builder
}

fn write_vec(src: &[u8], dst: &mut Vec<u8>, compression: Compression) {
	match compression {
		Compression::None => dst.extend_from_slice(src),
		#[cfg(feature = "lz4")]
		Compression::LZ4 => {
			let len = src.len() as i64;
			dst.extend_from_slice(&len.to_le_bytes());
			let mut encoder =
				lz4_flex::frame::FrameEncoder::new(dst);
			// writes to Vec<u8> are infallible
			encoder.write_all(src).unwrap();
			encoder.finish().unwrap();
		}
		#[cfg(feature = "zstd")]
		Compression::Zstd(level) => {
			let len = src.len() as i64;
			dst.extend_from_slice(&len.to_le_bytes());
			// the level is clamped
			let mut encoder =
				Encoder::new(dst, level.clamp(0, 22).into())
					.unwrap();
			// writes to Vec<u8> are infallible
			encoder.write_all(src).unwrap();
			encoder.finish().unwrap();
		}
	}
}

fn round_vec_len(data: &mut Vec<u8>) {
	let remaineder = data.len() % 8;
	if remaineder != 0 {
		data.resize(data.len() + 8 - remaineder, 0);
	}
}

fn write_batch<'a, 'fbb>(
	buf_metadata: Vec<u8>,
	mut buf_data: Vec<u8>,
	arrays: impl IntoIterator<Item = &'a dyn Array>,
	compression: Compression,
) -> Result<(FlatBufferBuilder<'fbb>, Vec<u8>)> {
	let mut builder = FlatBufferBuilder::from_vec(buf_metadata);

	let mut buffers = Vec::<Buffer>::new();
	let mut nodes = Vec::<FieldNode>::new();

	let mut num_rows: Option<usize> = None;
	for array in arrays.into_iter() {
		if let Some(num_rows) = num_rows {
			if num_rows != array.len() {
				return Err(Error::BatchDifferentLengths(
					(num_rows, array.len()).into(),
				));
			}
		} else {
			num_rows = Some(array.len());
		}

		array.walk_buffers(&mut |buf| {
			let offset = buf_data.len() as i64;
			write_vec(buf, &mut buf_data, compression);
			let length = buf_data.len() as i64 - offset;

			buffers.push(Buffer::new(offset, length));
			round_vec_len(&mut buf_data);
		});

		array.walk_nodes(&mut |length, null_count| {
			let length = length as i64;
			let null_count = null_count as i64;
			nodes.push(FieldNode::new(length, null_count));
		})
	}

	let compression = match compression {
		Compression::None => None,
		#[cfg(feature = "lz4")]
		Compression::LZ4 => Some(BodyCompression::create(
			&mut builder,
			&BodyCompressionArgs {
				codec: CompressionType::LZ4_FRAME,
				method: BodyCompressionMethod::BUFFER,
			},
		)),
		#[cfg(feature = "zstd")]
		Compression::Zstd(_) => Some(BodyCompression::create(
			&mut builder,
			&BodyCompressionArgs {
				codec: CompressionType::ZSTD,
				method: BodyCompressionMethod::BUFFER,
			},
		)),
	};

	let nodes = builder.create_vector(&nodes);
	let buffers = builder.create_vector(&buffers);

	let batch = RecordBatch::create(
		&mut builder,
		&RecordBatchArgs {
			length: num_rows.unwrap() as i64,
			nodes: Some(nodes),
			buffers: Some(buffers),
			compression,
			variadicBufferCounts: None,
		},
	)
	.as_union_value();

	let message = Message::create(
		&mut builder,
		&MessageArgs {
			version: MetadataVersion::V5,
			header: Some(batch),
			header_type: MessageHeader::RecordBatch,
			bodyLength: buf_data.len() as i64,
			custom_metadata: None,
		},
	);

	builder.finish(message, None);

	Ok((builder, buf_data))
}
