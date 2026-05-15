mod batch;
mod schema;
mod stream;

pub(crate) use batch::write_batch;
pub(crate) use schema::write_schema;

pub use stream::StreamWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
	None,
	LZ4,
	Zstd,
}
