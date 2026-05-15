pub trait ValidityBuffer {
	fn new() -> Self;

	fn push(&mut self, index: usize, valid: bool);

	/// Returns true if the bit at the given index is 0
	fn is_null(&self, index: usize) -> bool;
}

impl ValidityBuffer for () {
	fn new() -> Self {}

	fn push(&mut self, _index: usize, _valid: bool) {}

	fn is_null(&self, _index: usize) -> bool {
		false
	}
}

pub struct Bitmap(Vec<u8>);

impl ValidityBuffer for Bitmap {
	fn new() -> Self {
		Self(Vec::new())
	}

	fn push(&mut self, index: usize, valid: bool) {
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

	fn is_null(&self, index: usize) -> bool {
		let byte_index = index / 8;
		let bit_offset = index % 8;

		(self.0[byte_index] & (1 << bit_offset)) == 0
	}
}
