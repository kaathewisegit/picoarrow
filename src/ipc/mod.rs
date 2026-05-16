//! Arrow IPC serialization
//!
//! This module provides two writers:
//!
//! - [`StreamWriter`], which writes [the IPC streaming format][s]
//! - [`FileWriter`], a wrapper over `StreamWriter`, which writes [the IPC file
//!   format][f].
//!
//! Both writers will write header data into the destination on construction
//! with their `new` method.  Additionally, when writing to buffered writers
//! such as `BufWriter` or `File`, the `flush` method needs to be called on both
//! to finalize the buffers.
//!
//! [s]: https://arrow.apache.org/docs/format/Columnar.html#ipc-streaming-format
//! [f]: https://arrow.apache.org/docs/format/Columnar.html#ipc-file-format

mod file;
mod stream;

pub use file::FileWriter;
pub use stream::StreamWriter;

/// Compression type
///
/// The `Zstd` and `LZ4` variants must be enabled using the `zstd` and `lz4`
/// crate features.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
	None,
	#[cfg(feature = "lz4")]
	LZ4,
	/// ZSTD compression via the `zstd` crate
	///
	/// The inner item is the compression level.  Values outside of the
	/// support range (`[0-21]`) will be clamped, 0 selects the default
	/// level, which is 3 at the time of writing.
	#[cfg(feature = "zstd")]
	Zstd(u8),
}
