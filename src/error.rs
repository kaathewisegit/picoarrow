use std::{error, fmt, io::Error as IoError, str::Utf8Error};

use crate::schema::{DataType, Field};

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

	/// Errors returned by underlying readers used by the IPC deserializers
	ReadFailed(Box<IoError>),

	/// A flatbuffer in the stream could not be parsed
	InvalidFlatbuffer(Box<flatbuffers::InvalidFlatbuffer>),

	/// A message's header is not the type it is labeled as
	InvalidMessage,

	/// An IPC schema message's header could not be read
	InvalidSchemaMessage,

	/// A record batch message's header could not be read
	InvalidRecordBatchMessage,

	/// The first message in an IPC stream is not a schema
	FirstMessageNotSchema,

	/// An IPC schema message appears in the middle of the stream
	UnexpectedSchemaMessage,

	/// The IPC stream contains a message of an unsupported type
	UnsupportedMessageType,

	/// The IPC stream's body uses an unsupported compression codec
	UnsupportedCompression,

	/// IPC data contains a data type not representable by picoarrow
	UnsupportedDataType(Box<DataType>),

	/// The IPC stream ended before a schema message was found
	MissingSchema,

	/// The stream's continuation marker was malformed
	InvalidContinuationMarker,

	/// A IPC file's magic bytes are missing or malformed
	InvalidFileMagic,

	/// An IPC file's footer is missing or malformed
	InvalidFileFooter,

	/// The record batch refers to fewer buffers than the schema requires
	MissingBuffer,

	/// One of the record batch's buffers lies outside the message body
	InvalidBuffer,

	/// A variable-length array's offset buffer is too small
	InvalidOffsetBuffer,

	/// A variable-length array's offsets are out of order or out of bounds
	InvalidOffsets,

	/// A primitive array's values buffer is not a multiple of the element
	/// size
	InvalidValuesBuffer(Box<(usize, usize)>),

	/// An IPC record batch's node count doesn't match the schema
	FieldNodeCountMismatch(Box<(usize, usize)>),

	/// An IPC record batch's null count is larger than the batch length
	InvalidNullCount,

	/// A non-nullable field in IPC contains null values
	NullsInNonNullableField,

	/// When reading IPC array's values didn't match its reported length
	ArrayLengthMismatch,

	/// Length or offset from the IPC cannot be represented as `usize`
	InvalidLengthOrOffset,

	/// A UTF-8 column in IPC contains invalid UTF-8
	InvalidUtf8(Box<Utf8Error>),

	/// Used by [`FixedSizeList::push`][p] and [`ArrayStruct::push`][s]
	///
	/// [p]: super::array::ArrayFixedSizeList::push
	/// [s]: super::array::ArrayStruct::push
	WrongAppendLength(Box<(usize, usize)>),

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
	BatchSchemaMismatch(Box<(String, Field)>),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::WriteFailed(e) => {
				writeln!(f, "underlying writer failed: {e}")
			}
			Error::ReadFailed(e) => {
				writeln!(f, "underlying reader failed: {e}")
			}
			Error::InvalidFlatbuffer(e) => {
				writeln!(f, "invalid flatbuffer: {e}")
			}
			Error::InvalidMessage => {
				writeln!(f, "invalid message header")
			}
			Error::InvalidSchemaMessage => {
				writeln!(f, "invalid schema message header")
			}
			Error::InvalidRecordBatchMessage => {
				writeln!(
					f,
					"invalid record batch message header"
				)
			}
			Error::FirstMessageNotSchema => {
				writeln!(
					f,
					"the first message in the stream is not a schema"
				)
			}
			Error::UnexpectedSchemaMessage => {
				writeln!(
					f,
					"unexpected schema message in stream"
				)
			}
			Error::UnsupportedMessageType => {
				writeln!(f, "unsupported IPC message type")
			}
			Error::UnsupportedCompression => {
				writeln!(f, "unsupported body compression")
			}
			Error::UnsupportedDataType(type_) => {
				writeln!(f, "unsupported data type: {type_:?}")
			}
			Error::MissingSchema => {
				writeln!(
					f,
					"the stream ended before a schema was found"
				)
			}
			Error::InvalidContinuationMarker => {
				writeln!(f, "invalid IPC continuation marker")
			}
			Error::InvalidFileMagic => {
				writeln!(f, "invalid IPC file magic")
			}
			Error::InvalidFileFooter => {
				writeln!(f, "invalid IPC file footer")
			}
			Error::MissingBuffer => {
				writeln!(f, "missing IPC buffer")
			}
			Error::InvalidBuffer => {
				writeln!(f, "invalid IPC buffer")
			}
			Error::InvalidOffsetBuffer => {
				writeln!(f, "invalid IPC offset buffer")
			}
			Error::InvalidOffsets => {
				writeln!(f, "invalid IPC offsets")
			}
			Error::InvalidValuesBuffer(boxed) => {
				let (element_size, buffer_len) = **boxed;
				writeln!(
					f,
					"invalid IPC values buffer: buffer length {buffer_len} is not a multiple of the {element_size} byte element size"
				)
			}
			Error::FieldNodeCountMismatch(boxed) => {
				let (expected, got) = **boxed;
				writeln!(
					f,
					"expected {expected} field nodes, got {got}"
				)
			}
			Error::InvalidNullCount => {
				writeln!(f, "invalid null count")
			}
			Error::NullsInNonNullableField => {
				writeln!(f, "non-nullable field contains nulls")
			}
			Error::ArrayLengthMismatch => {
				writeln!(f, "array length mismatch")
			}
			Error::InvalidLengthOrOffset => {
				writeln!(f, "invalid IPC length or offset")
			}
			Error::InvalidUtf8(e) => {
				writeln!(f, "invalid UTF-8: {e}")
			}
			Error::WrongAppendLength(boxed) => {
				let (expected, got) = **boxed;
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
					"Expected field named {expected:?}, got {got:?}"
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
			Error::ReadFailed(err) => Some(err),
			Error::InvalidFlatbuffer(err) => Some(err),
			Error::InvalidUtf8(err) => Some(err),
			_ => None,
		}
	}
}

impl From<IoError> for Error {
	fn from(value: IoError) -> Self {
		Error::WriteFailed(value.into())
	}
}
