mod stream;

pub use stream::StreamWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
	None,
	LZ4,
	#[cfg(feature = "zstd")]
	Zstd(u8),
}
