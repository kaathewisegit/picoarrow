use flatbuffers::{FlatBufferBuilder, WIPOffset};
#[cfg(feature = "zstd")]
use zstd::stream::Encoder;

use std::{
	io::{Error as IoError, Write},
	mem::take,
};

use super::Compression;
use crate::{
	array::Array,
	fb::{
		BodyCompression, BodyCompressionArgs, BodyCompressionMethod,
		Buffer, CompressionType, Endianness, Feature, Field, FieldNode,
		Message, MessageArgs, MessageHeader, MetadataVersion,
		RecordBatch, RecordBatchArgs, Schema, SchemaArgs,
	},
};

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

	pub fn finish(mut self) -> Result<W, IoError> {
		self.writer
			.write_all(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0])?;
		self.writer.flush()?;
		Ok(self.writer)
	}
}

fn write_schema<'a, 'fbb>(
	buf_metadata: Vec<u8>,
	arrays: impl IntoIterator<Item = (&'a str, &'a dyn Array)>,
) -> FlatBufferBuilder<'fbb> {
	let mut builder = FlatBufferBuilder::from_vec(buf_metadata);
	let mut fields = Vec::<WIPOffset<Field<'fbb>>>::new();

	for (name, array) in arrays.into_iter() {
		let field = array.serialize_field(&mut builder, name);
		fields.push(field);
	}

	let features = builder.create_vector(&[Feature::COMPRESSED_BODY]);
	let fields = builder.create_vector(&fields);

	let schema = Schema::create(
		&mut builder,
		&SchemaArgs {
			endianness: Endianness::Little,
			custom_metadata: None,
			fields: Some(fields),
			features: Some(features),
		},
	)
	.as_union_value();

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
		Compression::LZ4 => unimplemented!(),
		#[cfg(feature = "zstd")]
		Compression::Zstd(level) => {
			let len = src.len() as i64;
			dst.extend_from_slice(&len.to_le_bytes());
			let mut encoder =
				Encoder::new(dst, level.clamp(0, 22).into())
					.unwrap();
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
) -> (FlatBufferBuilder<'fbb>, Vec<u8>) {
	let mut builder = FlatBufferBuilder::from_vec(buf_metadata);

	let mut buffers = Vec::<Buffer>::new();
	let mut nodes = Vec::<FieldNode>::new();

	let mut num_rows: Option<usize> = None;
	for array in arrays.into_iter() {
		if let Some(num_rows) = num_rows {
			assert_eq!(num_rows, array.len());
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

	(builder, buf_data)
}
