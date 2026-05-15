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

impl<'fbb> RecordBatchBuilder<'fbb> {
	pub fn new(
		mut builder: FlatBufferBuilder<'fbb>,
		compression: Compression,
	) -> Self {
		Self {
			compression,
			builder,
			metadata: RecordBatchArgs::default(),

			data: Vec::new(),
			nodes: Vec::new(),
			buffers: Vec::new(),
		}
	}

	pub fn add_array<A: Array>(&mut self, array: &A) {
		todo!()
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

fn serialize_batch(builder: &mut FlatBufferBuilder<'_>) {}
