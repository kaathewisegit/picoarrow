use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{
	array::Array,
	fb::{
		Endianness, Feature, Field, Message, MessageArgs,
		MessageHeader, MetadataVersion, Schema, SchemaArgs,
	},
};

pub struct SchemaBuilder<'fbb> {
	builder: FlatBufferBuilder<'fbb>,
	fields: Vec<WIPOffset<Field<'fbb>>>,
}

impl<'fbb> SchemaBuilder<'fbb> {
	pub fn new() -> Self {
		Self {
			builder: FlatBufferBuilder::new(),
			fields: Vec::new(),
		}
	}

	pub fn add_array<A: Array>(&mut self, name: &str, array: &A) {
		let field = array.serialize_field(&mut self.builder, name);
		self.fields.push(field);
	}

	pub fn finish(&mut self) -> &[u8] {
		let features =
			self.builder.create_vector(&[Feature::COMPRESSED_BODY]);
		let fields = self.builder.create_vector(&self.fields);

		let schema = Schema::create(
			&mut self.builder,
			&SchemaArgs {
				endianness: Endianness::Little,
				custom_metadata: None,
				fields: Some(fields),
				features: Some(features),
			},
		)
		.as_union_value();

		let message = Message::create(
			&mut self.builder,
			&MessageArgs {
				version: MetadataVersion::V5,
				header: Some(schema),
				header_type: MessageHeader::Schema,
				bodyLength: 0,
				custom_metadata: None,
			},
		);

		self.builder.finish(message, None);
		self.builder.finished_data()
	}
}

impl<'fbb> Default for SchemaBuilder<'fbb> {
	fn default() -> Self {
		Self::new()
	}
}
