use std::{error, fmt, io::Error as IoError};

/// The Result type of `picoarrow`'s functions and methods
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A union of all errors which can be returned by `picoarrow`
///
/// Some payloads will be boxed in order to ensure that the size of `Error` is
/// at most 16 bytes.
#[derive(Debug)]
pub enum Error {
	/// Errors returned by underlying writers used by the IPC serializers
	WriteFailed(Box<IoError>),

	/// Used by [`FixedSizeList::push`][p]
	///
	/// [p]: super::array::ArrayFixedSizeList::push
	WrongNestedLength {
		/// The child array must have been longer by this much after the
		/// call to `push`
		expected: i32,
		/// The actual increase in length of the child array
		got: usize,
	},

	/// Returned by IPC serializers when a batch has two arrays of different
	/// lengths.
	BatchDifferentLengths(Box<(usize, usize)>),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::WriteFailed(e) => {
				writeln!(f, "underlying writer failed: {e}")
			}
			Error::WrongNestedLength { expected, got } => {
				writeln!(
					f,
					"Tried to write {got} elements to a fixed sized list with the size of {expected}"
				)
			}
			Error::BatchDifferentLengths(boxed) => {
				let (a, b) = **boxed;
				writeln!(
					f,
					"The first array in the batch had a length of {a}, but there's another with the length of {b}"
				)
			}
		}
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Error::WriteFailed(err) => Some(err),
			_ => None,
		}
	}
}

impl From<IoError> for Error {
	fn from(value: IoError) -> Self {
		Error::WriteFailed(value.into())
	}
}
