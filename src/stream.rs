#![allow(unused)]

use flatbuffers::FlatBufferBuilder;

use crate::{
	array::Array,
	fb::{
		BodyCompression, BodyCompressionArgs, BodyCompressionMethod,
		Buffer, CompressionType, FieldNode, RecordBatch,
		RecordBatchArgs,
	},
};

pub enum Compression {
	None,
	LZ4,
	Zstd,
}

pub struct RecordBatchBuilder<'fbb> {
	compression: Compression,
	builder: FlatBufferBuilder<'fbb>,
	metadata: RecordBatchArgs<'fbb>,

	data: Vec<u8>,
	buffers: Vec<Buffer>,
	nodes: Vec<FieldNode>,
}

fn round_vec_len(data: &mut Vec<u8>) {
	let remaineder = data.len() % 8;
	if remaineder != 0 {
		data.resize(data.len() + 8 - remaineder, 0);
	}
}

impl<'fbb> RecordBatchBuilder<'fbb> {
	pub fn new(compression: Compression) -> Self {
		Self {
			compression,
			builder: FlatBufferBuilder::new(),
			metadata: RecordBatchArgs::default(),

			data: Vec::new(),
			nodes: Vec::new(),
			buffers: Vec::new(),
		}
	}

	pub fn add_array<A: Array>(&mut self, array: &A) {
		array.walk_buffers(|buf| {
			let offset = self.data.len() as i64;
			let length = buf.len() as i64;
			self.buffers.push(Buffer::new(offset, length));
			self.data.extend_from_slice(buf);
			round_vec_len(&mut self.data);
		});

		array.walk_nodes(|length, null_count| {
			let length = length as i64;
			let null_count = null_count as i64;
			self.nodes.push(FieldNode::new(length, null_count));
		})
	}

	pub fn finish(&mut self) -> &[u8] {
		let compression = match self.compression {
			Compression::None => None,
			Compression::LZ4 => Some(BodyCompression::create(
				&mut self.builder,
				&BodyCompressionArgs {
					codec: CompressionType::LZ4_FRAME,
					method: BodyCompressionMethod::BUFFER,
				},
			)),
			Compression::Zstd => Some(BodyCompression::create(
				&mut self.builder,
				&BodyCompressionArgs {
					codec: CompressionType::ZSTD,
					method: BodyCompressionMethod::BUFFER,
				},
			)),
		};

		let nodes = self.builder.create_vector(&self.nodes);
		let buffers = self.builder.create_vector(&self.buffers);

		let batch = RecordBatch::create(
			&mut self.builder,
			&RecordBatchArgs {
				length: 0,
				nodes: Some(nodes),
				buffers: Some(buffers),
				compression,
				variadicBufferCounts: None,
			},
		);

		self.builder.finish(batch, None);
		self.builder.finished_data()
	}
}
