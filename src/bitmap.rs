pub trait ValidityBuffer {
	fn new() -> Self;

	fn push(&mut self, index: usize, valid: bool);

	/// Returns true if the bit at the given index is 0
	fn is_null(&self, index: usize) -> bool;

	fn null_count(&self) -> usize;

	fn buffer(&self) -> &[u8];
}

impl ValidityBuffer for () {
	fn new() -> Self {}

	fn push(&mut self, _index: usize, _valid: bool) {}

	fn is_null(&self, _index: usize) -> bool {
		false
	}

	fn null_count(&self) -> usize {
		0
	}

	fn buffer(&self) -> &[u8] {
		&[]
	}
}

impl ValidityBuffer for Vec<u8> {
	fn new() -> Self {
		Vec::new()
	}

	fn push(&mut self, index: usize, valid: bool) {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		if byte_index >= self.len() {
			self.push(0);
		}

		if valid {
			self[byte_index] |= 1 << bit_offset;
		} else {
			self[byte_index] &= !(1 << bit_offset);
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

	fn buffer(&self) -> &[u8] {
		self
	}
}
