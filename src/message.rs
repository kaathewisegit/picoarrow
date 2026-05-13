use crate::schema::{KeyValue, MetadataVersion, Schema};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldNode {
	pub length: i64,
	pub null_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionType {
	Lz4Frame,
	Zstd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyCompression {
	pub codec: CompressionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Buffer {
	offset: i64,
	length: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordBatch {
	length: i64,
	nodes: Vec<FieldNode>,
	buffers: Vec<Buffer>,
	compression: Option<BodyCompression>,
	variadic_buffer_counts: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageHeader {
	Schema(Schema),
	RecordBatch(RecordBatch),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Message {
	version: MetadataVersion,
	header: MessageHeader,
	body_length: i64,
	custom_metadata: Vec<KeyValue>,
}
