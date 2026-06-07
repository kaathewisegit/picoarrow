use std::{error, fmt, io::Error as IoError};

use crate::schema::Field;

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

	/// Used by [`FixedSizeList::push`][p] and [`ArrowStruct::push`][s]
	///
	/// [p]: super::array::ArrayFixedSizeList::push
	/// [s]: super::array::ArrowStruct::push
	WrongAppendLength {
		expected: usize,
		got: usize,
	},

	WrongBinaryAppendLength {
		expected: i32,
		got: usize,
	},

	/// Returned by variable-length arrays when the total data size would
	/// overflow the offset type
	LengthOverflow,

	/// Returned by IPC serializers when a batch has two arrays of different
	/// lengths.
	BatchDifferentLengths(Box<(usize, usize)>),

	/// Tried to serialize a different number of arrays than there were in
	/// the schema
	BatchDifferentNumberOfArrays {
		expected: u32,
		got: u32,
	},

	/// Returned by IPC serializers when arrays in a batch don't match the
	/// schema.
	BatchSchemaMismatch(Box<(Field, Field)>),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::WriteFailed(e) => {
				writeln!(f, "underlying writer failed: {e}")
			}
			Error::WrongAppendLength { expected, got } => {
				writeln!(
					f,
					"Wrong append length: expected {expected}, got {got}"
				)
			}
			Error::WrongBinaryAppendLength { expected, got } => {
				writeln!(
					f,
					"Tried to write {got} bytes to a fixed sized binary array, expected {expected}"
				)
			}
			Error::LengthOverflow => {
				writeln!(
					f,
					"Offset overflow: total data size exceeds 2^31 - 1"
				)
			}
			Error::BatchDifferentNumberOfArrays {
				expected,
				got,
			} => {
				writeln!(
					f,
					"Expected {expected} arrays in a batch, got {got}"
				)
			}
			Error::BatchSchemaMismatch(boxed) => {
				let (expected, got) = boxed.as_ref();
				writeln!(
					f,
					"Expected field {expected:?}, got {got:?}"
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
