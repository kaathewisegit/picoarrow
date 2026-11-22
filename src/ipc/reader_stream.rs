// TODO: array constructors which take raw buffers to avoid intermediary copies

use bytemuck::cast_slice;

#[cfg(feature = "lz4")]
use lz4_flex::frame::FrameDecoder;

#[cfg(feature = "zstd")]
use zstd::stream::Decoder;

use std::io::{Error as IoError, ErrorKind, Read};

#[cfg(any(feature = "lz4", feature = "zstd"))]
use crate::fb::CompressionType;

use crate::{
	Error, Result,
	array::{
		Array, ArrayBinary, ArrayBoolean, ArrayFixedBinary,
		ArrayPrimitive, ArrayUtf8, NonNullable, Nullable,
	},
	fb::{Buffer, MessageHeader, Precision, root_as_message},
	schema::{DataType, Field, Schema},
};

/// A deserialized column
#[derive(Debug, PartialEq)]
pub enum Column {
	Boolean(ArrayBoolean<Nullable>),
	BooleanNonNullable(ArrayBoolean<NonNullable>),
	I8(ArrayPrimitive<i8, Nullable>),
	I8NonNullable(ArrayPrimitive<i8, NonNullable>),
	I16(ArrayPrimitive<i16, Nullable>),
	I16NonNullable(ArrayPrimitive<i16, NonNullable>),
	I32(ArrayPrimitive<i32, Nullable>),
	I32NonNullable(ArrayPrimitive<i32, NonNullable>),
	I64(ArrayPrimitive<i64, Nullable>),
	I64NonNullable(ArrayPrimitive<i64, NonNullable>),
	U8(ArrayPrimitive<u8, Nullable>),
	U8NonNullable(ArrayPrimitive<u8, NonNullable>),
	U16(ArrayPrimitive<u16, Nullable>),
	U16NonNullable(ArrayPrimitive<u16, NonNullable>),
	U32(ArrayPrimitive<u32, Nullable>),
	U32NonNullable(ArrayPrimitive<u32, NonNullable>),
	U64(ArrayPrimitive<u64, Nullable>),
	U64NonNullable(ArrayPrimitive<u64, NonNullable>),
	#[cfg(feature = "half")]
	F16(ArrayPrimitive<half::f16, Nullable>),
	#[cfg(feature = "half")]
	F16NonNullable(ArrayPrimitive<half::f16, NonNullable>),
	F32(ArrayPrimitive<f32, Nullable>),
	F32NonNullable(ArrayPrimitive<f32, NonNullable>),
	F64(ArrayPrimitive<f64, Nullable>),
	F64NonNullable(ArrayPrimitive<f64, NonNullable>),
	Binary(ArrayBinary<Nullable>),
	BinaryNonNullable(ArrayBinary<NonNullable>),
	Utf8(ArrayUtf8<Nullable>),
	Utf8NonNullable(ArrayUtf8<NonNullable>),
	FixedBinary(ArrayFixedBinary<Nullable>),
	FixedBinaryNonNullable(ArrayFixedBinary<NonNullable>),
}

/// An IPC reader for [the streaming format][s]
///
/// Dictionaries are not supported.
///
/// [s]: https://arrow.apache.org/docs/format/Columnar.html#ipc-streaming-format
pub struct StreamReader<R> {
	reader: R,
	schema: Schema,
	read_eos: bool,
}

impl<R: Read> StreamReader<R> {
	/// Creates a reader and reads its schema
	pub fn new(mut reader: R) -> Result<Self> {
		let Some((metadata, _)) = read_raw_message(&mut reader)? else {
			return Err(Error::MissingSchema);
		};

		let message = root_as_message(&metadata)
			.map_err(|e| Error::InvalidFlatbuffer(Box::new(e)))?;
		let MessageHeader::Schema = message.header_type() else {
			return Err(Error::FirstMessageNotSchema);
		};
		let Some(schema) = message.header_as_schema() else {
			return Err(Error::InvalidSchemaMessage);
		};
		let schema = Schema::deserialize(&schema);

		Ok(Self {
			reader,
			schema,
			read_eos: false,
		})
	}

	/// Returns the schema of the stream
	pub fn schema(&self) -> &Schema {
		&self.schema
	}

	/// Returns the backing reader
	pub fn into_inner(self) -> R {
		self.reader
	}
}

/// Reads the next record batch
///
/// Returns `None` once the end-of-stream marker has been reached.
impl<R: Read> Iterator for StreamReader<R> {
	type Item = Result<Vec<Column>>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.read_eos {
			return None;
		}

		let (metadata, body) = match read_raw_message(&mut self.reader)
		{
			Ok(Some(message)) => message,
			Ok(None) => {
				self.read_eos = true;
				return None;
			}
			Err(e) => return Some(Err(e)),
		};

		let Ok(message) = root_as_message(&metadata) else {
			return Some(Err(Error::InvalidMessage));
		};
		Some(match message.header_type() {
			MessageHeader::Schema => {
				Err(Error::UnexpectedSchemaMessage)
			}
			MessageHeader::RecordBatch => {
				let Some(batch) =
					message.header_as_record_batch()
				else {
					return Some(Err(
						Error::InvalidRecordBatchMessage,
					));
				};
				decode_batch(&self.schema, &batch, &body)
			}
			_ => Err(Error::UnsupportedMessageType),
		})
	}
}

// TODO: reuse buffers instead of returning newly allocated Vec<u8>
fn read_raw_message<R: Read>(
	reader: &mut R,
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	let Some(prefix) = read_exact_opt(reader)? else {
		return Ok(None);
	};

	if prefix != [0xff; 4] {
		return Err(Error::InvalidContinuationMarker);
	}

	let mut len_buf = [0; 4];
	reader.read_exact(&mut len_buf)
		.map_err(|e| Error::ReadFailed(e.into()))?;
	let len = i32::from_le_bytes(len_buf);

	if len == 0 {
		return Ok(None);
	}

	let mut metadata = vec![0; usize_from(i64::from(len))?];
	reader.read_exact(&mut metadata)
		.map_err(|e| Error::ReadFailed(e.into()))?;

	let message = root_as_message(&metadata)
		.map_err(|e| Error::InvalidFlatbuffer(Box::new(e)))?;
	let mut body = vec![0; usize_from(message.bodyLength())?];
	reader.read_exact(&mut body)
		.map_err(|e| Error::ReadFailed(e.into()))?;

	// Validation of the message type is left to the callers, which must keep
	// `metadata` alive while its flatbuffers are in use.
	Ok(Some((metadata, body)))
}

/// Reads four bytes, returning `None` only if the stream ended before the
/// first byte.
fn read_exact_opt<R: Read>(reader: &mut R) -> Result<Option<[u8; 4]>> {
	let mut buf = [0; 4];
	let mut filled = 0;

	while filled < buf.len() {
		match reader.read(&mut buf[filled..]).map_err(read_failed)? {
			0 => {
				return if filled == 0 {
					Ok(None)
				} else {
					Err(unexpected_end())
				};
			}
			n => filled += n,
		}
	}

	Ok(Some(buf))
}

fn read_failed(err: IoError) -> Error {
	Error::ReadFailed(err.into())
}

fn unexpected_end() -> Error {
	IoError::new(ErrorKind::UnexpectedEof, "unexpected end of stream")
		.into()
}

#[derive(Clone, Copy)]
enum Codec {
	None,
	#[cfg(feature = "lz4")]
	Lz4,
	#[cfg(feature = "zstd")]
	Zstd,
}

impl Codec {
	fn from_batch(batch: &crate::fb::RecordBatch<'_>) -> Result<Self> {
		let Some(compression) = batch.compression() else {
			return Ok(Self::None);
		};

		match compression.codec() {
			#[cfg(feature = "lz4")]
			CompressionType::LZ4_FRAME => Ok(Self::Lz4),
			#[cfg(feature = "zstd")]
			CompressionType::ZSTD => Ok(Self::Zstd),
			_ => Err(Error::UnsupportedCompression),
		}
	}
}

struct Body<'a> {
	buffers: Vec<Buffer>,
	body: &'a [u8],
	codec: Codec,
	cursor: usize,
}

impl<'a> Body<'a> {
	fn new(
		buffers: Option<flatbuffers::Vector<'a, Buffer>>,
		body: &'a [u8],
		codec: Codec,
	) -> Self {
		Self {
			buffers: buffers
				.map(|b| b.iter().copied().collect())
				.unwrap_or_default(),
			body,
			codec,
			cursor: 0,
		}
	}

	fn next(&mut self) -> Result<Vec<u8>> {
		let buffer = self
			.buffers
			.get(self.cursor)
			.copied()
			.ok_or(Error::MissingBuffer)?;
		self.cursor += 1;

		let offset = usize_from(buffer.offset())?;
		let length = usize_from(buffer.length())?;
		let end = offset
			.checked_add(length)
			.filter(|end| *end <= self.body.len())
			.ok_or(Error::InvalidBuffer)?;

		let buf = &self.body[offset..end];
		match self.codec {
			Codec::None => Ok(buf.to_vec()),
			#[cfg(feature = "lz4")]
			Codec::Lz4 => {
				if length == 0 {
					return Ok(Vec::new());
				}
				let data = prefixed_data(buf)?;
				decompress_all(FrameDecoder::new(data))
					.map_err(read_failed)
			}
			#[cfg(feature = "zstd")]
			Codec::Zstd => {
				if length == 0 {
					return Ok(Vec::new());
				}
				let data = prefixed_data(buf)?;
				decompress_all(
					Decoder::new(data)
						.map_err(read_failed)?,
				)
				.map_err(read_failed)
			}
		}
	}
}

/// Strip length prefix from buffers
#[cfg(any(feature = "lz4", feature = "zstd"))]
fn prefixed_data(buf: &[u8]) -> Result<&[u8]> {
	let _ = i64::from_le_bytes(buf[..8].try_into().unwrap());
	Ok(&buf[8..])
}

fn bit(buffer: &[u8], index: usize) -> bool {
	buffer.get(index / 8)
		.is_some_and(|byte| byte & (1 << (index % 8)) != 0)
}

fn offsets(buffer: &[u8], len: usize) -> Result<&[[u8; 4]]> {
	if buffer.len() < (len + 1) * 4 {
		return Err(Error::InvalidOffsetBuffer);
	}
	Ok(cast_slice::<u8, [u8; 4]>(buffer))
}

fn decode_batch(
	schema: &Schema,
	batch: &crate::fb::RecordBatch<'_>,
	body: &[u8],
) -> Result<Vec<Column>> {
	let len = usize_from(batch.length())?;
	let codec = Codec::from_batch(batch)?;
	let mut body = Body::new(batch.buffers(), body, codec);

	let mut nodes = batch.nodes().map_or_else(Vec::new, |nodes| {
		nodes.iter().collect::<Vec<_>>()
	});
	if nodes.len() != schema.fields.len() {
		return Err(Error::FieldNodeCountMismatch(Box::new((
			schema.fields.len(),
			nodes.len(),
		))));
	}

	let mut columns = Vec::with_capacity(schema.fields.len());
	for field in &schema.fields {
		let node = nodes.remove(0);
		columns.push(decode_column(
			field,
			len,
			node.null_count(),
			&mut body,
		)?);
	}

	Ok(columns)
}

macro_rules! decode_primitive {
	($ty:ty, $nullable:ident, $variant:ident, $variant_non_null:ident, $body:expr, $len:expr, $null_count:expr, $field:expr) => {{
		let validity = $body.next()?;
		let values_buf = $body.next()?;
		let values: Vec<$ty> = if values_buf.len() % size_of::<$ty>()
			== 0
		{
			cast_slice(&values_buf).to_vec()
		} else {
			return Err(Error::InvalidValuesBuffer(Box::new((
				size_of::<$ty>(),
				values_buf.len(),
			))));
		};

		if $field.nullable {
			let mut arr = ArrayPrimitive::<$ty, Nullable>::new();
			for (i, value) in values.into_iter().enumerate() {
				if bit(&validity, i) {
					arr.push_some(value);
				} else {
					arr.push_null();
				}
			}
			Ok(Column::$variant(arr))
		} else if $null_count != 0 {
			return Err(Error::NullsInNonNullableField);
		} else {
			let mut arr = ArrayPrimitive::<$ty, NonNullable>::new();
			for value in values {
				arr.push(value);
			}
			Ok(Column::$variant_non_null(arr))
		}
	}};
}

fn decode_column(
	field: &Field,
	len: usize,
	null_count: i64,
	body: &mut Body<'_>,
) -> Result<Column> {
	if usize_from(null_count)? > len {
		return Err(Error::InvalidNullCount);
	}

	let type_ = &field.type_;
	match type_ {
		DataType::Bool => {
			let validity = body.next()?;
			let values = body.next()?;
			if field.nullable {
				let mut arr = ArrayBoolean::<Nullable>::new();
				for i in 0..len {
					if bit(&validity, i) {
						arr.push_some(bit(&values, i));
					} else {
						arr.push_null();
					}
				}
				Ok(Column::Boolean(arr))
			} else if null_count != 0 {
				Err(Error::NullsInNonNullableField)
			} else {
				let mut arr =
					ArrayBoolean::<NonNullable>::new();
				for i in 0..len {
					arr.push(bit(&values, i));
				}
				Ok(Column::BooleanNonNullable(arr))
			}
		}
		DataType::Binary => {
			let validity = body.next()?;
			let offsets = offsets(&body.next()?, len)?.to_vec();
			let values = body.next()?;
			let mut arr = ArrayBinary::<Nullable>::new();

			for i in 0..len {
				let start = usize_from(i64::from(
					i32::from_le_bytes(offsets[i]),
				))?;
				let end = usize_from(i64::from(
					i32::from_le_bytes(offsets[i + 1]),
				))?;
				let value = slice(&values, start, end)?;
				if field.nullable && !bit(&validity, i) {
					arr.push_null();
				} else {
					arr.push_some(value)?;
				}
			}

			if !field.nullable && null_count != 0 {
				return Err(Error::NullsInNonNullableField);
			}

			Ok(if field.nullable {
				Column::Binary(arr)
			} else {
				Column::BinaryNonNullable(to_non_null_binary(
					arr,
				)?)
			})
		}
		DataType::Utf8 => {
			let validity = body.next()?;
			let offsets = offsets(&body.next()?, len)?.to_vec();
			let values = body.next()?;
			let mut arr = ArrayUtf8::<Nullable>::new();

			for i in 0..len {
				let start = usize_from(i64::from(
					i32::from_le_bytes(offsets[i]),
				))?;
				let end = usize_from(i64::from(
					i32::from_le_bytes(offsets[i + 1]),
				))?;
				let value = slice(&values, start, end)?;
				if field.nullable && !bit(&validity, i) {
					arr.push_null();
				} else {
					let value =
						std::str::from_utf8(value)
							.map_err(|e| {
								Error::InvalidUtf8(e.into())
							})?;
					arr.push_some(value)?;
				}
			}

			if !field.nullable && null_count != 0 {
				return Err(Error::NullsInNonNullableField);
			}

			Ok(if field.nullable {
				Column::Utf8(arr)
			} else {
				Column::Utf8NonNullable(to_non_null_utf8(arr)?)
			})
		}
		DataType::FixedSizeBinary { byte_width } => {
			let validity = body.next()?;
			let values = body.next()?;
			let width = usize_from(i64::from(*byte_width))?;
			let mut arr =
				ArrayFixedBinary::<Nullable>::new(*byte_width);

			for i in 0..len {
				let start = i
					.checked_mul(width)
					.ok_or(Error::LengthOverflow)?;
				let value =
					slice(&values, start, start + width)?;
				if field.nullable && !bit(&validity, i) {
					arr.push_null();
				} else {
					arr.push_some(value)?;
				}
			}

			if !field.nullable && null_count != 0 {
				return Err(Error::NullsInNonNullableField);
			}

			Ok(if field.nullable {
				Column::FixedBinary(arr)
			} else {
				Column::FixedBinaryNonNullable(
					to_non_null_fixed_binary(arr)?,
				)
			})
		}
		DataType::Int {
			bit_width,
			is_signed,
		} => match (*bit_width, *is_signed) {
			(8, true) => decode_primitive!(
				i8,
				Nullable,
				I8,
				I8NonNullable,
				body,
				len,
				null_count,
				field
			),
			(16, true) => decode_primitive!(
				i16,
				Nullable,
				I16,
				I16NonNullable,
				body,
				len,
				null_count,
				field
			),
			(32, true) => decode_primitive!(
				i32,
				Nullable,
				I32,
				I32NonNullable,
				body,
				len,
				null_count,
				field
			),
			(64, true) => decode_primitive!(
				i64,
				Nullable,
				I64,
				I64NonNullable,
				body,
				len,
				null_count,
				field
			),
			(8, false) => decode_primitive!(
				u8,
				Nullable,
				U8,
				U8NonNullable,
				body,
				len,
				null_count,
				field
			),
			(16, false) => decode_primitive!(
				u16,
				Nullable,
				U16,
				U16NonNullable,
				body,
				len,
				null_count,
				field
			),
			(32, false) => decode_primitive!(
				u32,
				Nullable,
				U32,
				U32NonNullable,
				body,
				len,
				null_count,
				field
			),
			(64, false) => decode_primitive!(
				u64,
				Nullable,
				U64,
				U64NonNullable,
				body,
				len,
				null_count,
				field
			),
			_ => Err(unsupported(type_)),
		},
		DataType::FloatingPoint { precision } => match *precision {
			Precision::HALF => {
				#[cfg(feature = "half")]
				{
					decode_primitive!(
						half::f16,
						Nullable,
						F16,
						F16NonNullable,
						body,
						len,
						null_count,
						field
					)
				}
				#[cfg(not(feature = "half"))]
				Err(unsupported(type_))
			}
			Precision::SINGLE => decode_primitive!(
				f32,
				Nullable,
				F32,
				F32NonNullable,
				body,
				len,
				null_count,
				field
			),
			Precision::DOUBLE => decode_primitive!(
				f64,
				Nullable,
				F64,
				F64NonNullable,
				body,
				len,
				null_count,
				field
			),
			_ => Err(unsupported(type_)),
		},
		_ => Err(unsupported(type_)),
	}
}

fn slice(buffer: &[u8], start: usize, end: usize) -> Result<&[u8]> {
	if start > end {
		return Err(Error::InvalidOffsets);
	}
	buffer.get(start..end).ok_or(Error::InvalidOffsets)
}

fn unsupported(data_type: &DataType) -> Error {
	Error::UnsupportedDataType(Box::new(data_type.clone()))
}

fn to_non_null_binary(
	arr: ArrayBinary<Nullable>,
) -> Result<ArrayBinary<NonNullable>> {
	if arr.null_count() != 0 {
		return Err(Error::NullsInNonNullableField);
	}
	let mut result = ArrayBinary::<NonNullable>::new();
	for i in 0..arr.len() {
		let Some(value) = arr.get(i) else {
			return Err(Error::ArrayLengthMismatch);
		};
		result.push(value)?;
	}
	Ok(result)
}

// TODO: remove
fn to_non_null_utf8(
	arr: ArrayUtf8<Nullable>,
) -> Result<ArrayUtf8<NonNullable>> {
	if arr.null_count() != 0 {
		return Err(Error::NullsInNonNullableField);
	}
	let mut result = ArrayUtf8::<NonNullable>::new();
	for i in 0..arr.len() {
		let Some(value) = arr.get(i) else {
			return Err(Error::ArrayLengthMismatch);
		};
		result.push(value)?;
	}
	Ok(result)
}

fn to_non_null_fixed_binary(
	arr: ArrayFixedBinary<Nullable>,
) -> Result<ArrayFixedBinary<NonNullable>> {
	if arr.null_count() != 0 {
		return Err(Error::NullsInNonNullableField);
	}
	let byte_width = arr.get(0).map_or(1, |value| value.len());
	let byte_width = i32::try_from(byte_width).unwrap_or(1);
	let mut result = ArrayFixedBinary::<NonNullable>::new(byte_width);
	for i in 0..arr.len() {
		let Some(value) = arr.get(i) else {
			return Err(Error::ArrayLengthMismatch);
		};
		result.push(value)?;
	}
	Ok(result)
}

fn usize_from(value: i64) -> Result<usize> {
	usize::try_from(value).map_err(|_| Error::InvalidLengthOrOffset)
}

#[cfg(any(feature = "lz4", feature = "zstd"))]
fn decompress_all(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
	let mut result = Vec::new();
	reader.read_to_end(&mut result)?;
	Ok(result)
}
