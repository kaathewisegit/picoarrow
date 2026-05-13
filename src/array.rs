use crate::schema::Field;

pub struct Bitmap(Vec<u8>);

impl Bitmap {
	pub fn new() -> Self {
		Self(Vec::new())
	}

	pub fn push(&mut self, index: usize, valid: bool) {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		if byte_index >= self.0.len() {
			self.0.push(0);
		}

		if valid {
			self.0[byte_index] |= 1 << bit_offset;
		} else {
			self.0[byte_index] &= !(1 << bit_offset);
		}
	}

	/// Returns true if the bit at the given index is 0
	pub fn is_null(&self, index: usize) -> bool {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		(self.0[byte_index] & (1 << bit_offset)) == 0
	}
}

pub struct RawArray {
	length: usize,

	nulls: Option<Bitmap>,

	buffers: Box<[Vec<u8>]>,
	children: Box<[RawArray]>,
}

impl RawArray {
	pub fn from_field(field: &Field) -> Self {
		let nulls = if field.nullable {
			Some(Bitmap::new())
		} else {
			None
		};

		let buffers = vec![Vec::new(); field.data_type.num_buffers()]
			.into_boxed_slice();

		let children = field
			.children
			.iter()
			.map(RawArray::from_field)
			.collect();

		Self {
			length: 0,
			nulls,
			buffers,
			children,
		}
	}

	pub fn is_null(&self, index: usize) -> bool {
		assert!(index < self.length);
		self.nulls
			.as_ref()
			.map(|b| b.is_null(index))
			.unwrap_or(true)
	}

	pub fn child(&self, index: usize) -> &RawArray {
		&self.children[index]
	}

	pub fn child_mut(&mut self, index: usize) -> &mut RawArray {
		&mut self.children[index]
	}

	pub fn push_bytes(&mut self, buf: usize, bytes: &[u8]) {
		self.buffers[buf].extend_from_slice(bytes);
	}

	pub fn push_validity(&mut self, valid: bool) {
		if let Some(b) = self.nulls.as_mut() {
			b.push(self.length, valid)
		} else if valid {
			panic!("Tried to push null to a non-nullable array");
		}
	}

	pub fn push_offset(&mut self, buf: usize, offset: usize) {
		let offset = offset as u32;
		self.buffers[buf].extend_from_slice(&offset.to_le_bytes());
	}

	pub fn increment_length(&mut self) {
		self.length += 1;
	}
}

macro_rules! push_primitive {
	($name:ident, $type:ty) => {
		pub fn $name(&mut self, value: $type) {
			self.push_bytes(0, &value.to_le_bytes());
			self.push_validity(true);
			self.increment_length();
		}
	};
}

impl RawArray {
	push_primitive!(push_u8, u8);
	push_primitive!(push_u16, u16);
	push_primitive!(push_u32, u32);
	push_primitive!(push_u64, u64);

	push_primitive!(push_i8, i8);
	push_primitive!(push_i16, i16);
	push_primitive!(push_i32, i32);
	push_primitive!(push_i64, i64);

	push_primitive!(push_f32, f32);
	push_primitive!(push_f64, f64);
}

macro_rules! push_variable_len {
	($name:ident, $type:ty) => {
		pub fn $name(&mut self, value: &$type) {
			// offset
			if self.length == 0 {
				self.push_offset(0, 0);
			}
			self.push_offset(
				1,
				value.len() + self.buffers[0].len(),
			);

			// contents
			self.push_bytes(1, value.as_ref());

			self.push_validity(true);
			self.increment_length();
		}
	};
}

impl RawArray {
	push_variable_len!(push_binary, [u8]);
	push_variable_len!(push_str, str);
}
