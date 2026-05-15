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
