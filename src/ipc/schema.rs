use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{
	array::Array,
	fb::{
		Endianness, Feature, Field, Message, MessageArgs,
		MessageHeader, MetadataVersion, Schema, SchemaArgs,
	},
};

pub fn write_schema<'a, 'fbb>(
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
