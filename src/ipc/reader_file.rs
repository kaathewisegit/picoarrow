use std::io::Read;

use crate::{Error, Result, Schema};

use super::StreamReader;
use crate::fb::Footer;

/// An IPC reader for [the file format][f]
///
/// Dictionaries are not supported.
///
/// [f]: https://arrow.apache.org/docs/format/Columnar.html#ipc-file-format
pub struct FileReader<R> {
	stream: StreamReader<R>,
	footer_checked: bool,
}

impl<R: Read> FileReader<R> {
	pub fn new(mut reader: R) -> Result<Self> {
		let mut magic = [0; 8];
		reader.read_exact(&mut magic)
			.map_err(|_| Error::InvalidFileMagic)?;
		if &magic != b"ARROW1\0\0" {
			return Err(Error::InvalidFileMagic);
		}

		let stream = StreamReader::new(reader)?;

		Ok(Self {
			stream,
			footer_checked: false,
		})
	}

	pub fn schema(&self) -> &Schema {
		self.stream.schema()
	}

	pub fn into_inner(self) -> R {
		self.stream.into_inner()
	}
}

impl<R: Read> Iterator for FileReader<R> {
	type Item = Result<Vec<super::Column>>;

	fn next(&mut self) -> Option<Self::Item> {
		let result = self.stream.next();

		if result.is_none() && !self.footer_checked {
			self.footer_checked = true;
			if let Err(e) = check_footer(
				&mut self.stream.reader,
				&self.stream.schema,
			) {
				return Some(Err(e));
			}
		}

		result
	}
}

/// Reads the footer at the current position of the reader
///
/// The footer must be less than 1MB large to avoid OOMs.
fn check_footer<R: Read>(reader: &mut R, schema: &Schema) -> Result<()> {
	let mut buf = Vec::new();
	reader.take(1024 * 1024).read_to_end(&mut buf)?;

	if !buf.ends_with(b"ARROW1") {
		return Err(Error::InvalidFileFooter);
	}
	buf.truncate(buf.len() - 6);

	if buf.len() < 4 {
		return Err(Error::InvalidFileFooter);
	}
	let footer_len = i32::from_le_bytes(*buf.last_chunk::<4>().unwrap());
	buf.truncate(buf.len() - 4);
	if footer_len.is_negative() {
		return Err(Error::InvalidFileFooter);
	}

	if buf.len() != footer_len as usize {
		return Err(Error::InvalidFileFooter);
	}

	let footer = flatbuffers::root::<Footer>(&buf)
		.map_err(|_| Error::InvalidFileFooter)?;
	let Some(fb_schema) = footer.schema() else {
		return Err(Error::InvalidFileFooter);
	};

	if schema != &Schema::deserialize(&fb_schema) {
		return Err(Error::InvalidFileFooter);
	}

	Ok(())
}
