pub trait ValidityBuffer {
	fn new() -> Self;

	fn resize_bits(&mut self, len: usize);

	fn set_bit(&mut self, index: usize, value: bool);

	fn push_many(&mut self, index: usize, valid: bool, num: usize);

	/// Returns true if the bit at the given index is 0
	fn is_null(&self, index: usize) -> bool;

	fn null_count(&self) -> usize;

	fn memory_size(&self) -> usize;

	fn clear(&mut self);

	fn shrink_to_fit(&mut self);

	fn buffer(&self) -> &[u8];
}

impl ValidityBuffer for () {
	fn new() -> Self {}

	fn resize_bits(&mut self, _len: usize) {}

	fn set_bit(&mut self, _index: usize, _value: bool) {}

	fn push_many(&mut self, _index: usize, _valid: bool, _num: usize) {}

	fn is_null(&self, _index: usize) -> bool {
		false
	}

	fn null_count(&self) -> usize {
		0
	}

	fn memory_size(&self) -> usize {
		0
	}

	fn clear(&mut self) {}

	fn shrink_to_fit(&mut self) {}

	fn buffer(&self) -> &[u8] {
		&[]
	}
}

impl ValidityBuffer for Vec<u8> {
	fn new() -> Self {
		Vec::new()
	}

	fn resize_bits(&mut self, len: usize) {
		self.resize(len.div_ceil(8), 0);
	}

	fn set_bit(&mut self, index: usize, value: bool) {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		if value {
			self[byte_index] |= 1 << bit_offset;
		} else {
			self[byte_index] &= !(1 << bit_offset);
		}
	}

	fn push_many(&mut self, index: usize, valid: bool, num: usize) {
		if num == 0 {
			return;
		}

		let start_bit = index;
		let end_bit = start_bit + num;
		let len_new = end_bit.div_ceil(8);
		self.resize(len_new, 0);

		if valid {
			let first_byte = start_bit / 8;
			let last_byte = end_bit / 8;

			let first_offset = start_bit % 8;
			if first_offset != 0 {
				let first_count = 8 - first_offset;
				self[first_byte] |= (1u8 << first_count) - 1;
			}

			let middle_start =
				first_byte + usize::from(first_offset != 0);
			let middle_end = last_byte;
			for byte in &mut self[middle_start..middle_end] {
				*byte = 0xFF;
			}

			let last_count = end_bit % 8;
			if last_count != 0 {
				self[last_byte] |= (1u8 << last_count) - 1;
			}
		}
	}

	fn is_null(&self, index: usize) -> bool {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		(self[byte_index] & (1 << bit_offset)) == 0
	}

	fn null_count(&self) -> usize {
		self.iter().map(|&b| b.count_ones() as usize).sum()
	}

	fn memory_size(&self) -> usize {
		self.len()
	}

	fn clear(&mut self) {
		self.clear()
	}

	fn shrink_to_fit(&mut self) {
		self.shrink_to_fit()
	}

	fn buffer(&self) -> &[u8] {
		self
	}
}
