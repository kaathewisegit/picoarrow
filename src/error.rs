use std::{error, fmt, io::Error as IoError};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
	WriteFailed(Box<IoError>),
	WrongNestedLength { expected: i32, got: usize },
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
