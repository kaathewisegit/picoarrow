mod file;
mod stream;

pub use file::FileWriter;
pub use stream::StreamWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
	None,
	#[cfg(feature = "lz4")]
	LZ4,
	#[cfg(feature = "zstd")]
	Zstd(u8),
}
