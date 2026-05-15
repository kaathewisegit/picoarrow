use flatbuffers::FlatBufferBuilder;

use super::Compression;
use crate::{
	array::Array,
	fb::{
		BodyCompression, BodyCompressionArgs, BodyCompressionMethod,
		Buffer, CompressionType, FieldNode, Message, MessageArgs,
		MessageHeader, MetadataVersion, RecordBatch, RecordBatchArgs,
	},
};

fn round_vec_len(data: &mut Vec<u8>) {
	let remaineder = data.len() % 8;
	if remaineder != 0 {
		data.resize(data.len() + 8 - remaineder, 0);
	}
}

pub fn write_batch<'a, 'fbb>(
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
			let length = buf.len() as i64;
			buffers.push(Buffer::new(offset, length));
			buf_data.extend_from_slice(buf);
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
		Compression::Zstd => Some(BodyCompression::create(
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
