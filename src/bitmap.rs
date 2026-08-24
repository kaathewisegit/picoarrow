pub trait ValidityBuffer {
	fn new() -> Self;

	fn resize_bits(&mut self, capacity: usize);

	fn set_bit_on(&mut self, index: usize);

	fn set_bits_on(&mut self, start: usize, num: usize);

	fn get(&self, index: usize) -> bool;

	fn count_ones(&self) -> usize;

	fn memory_size(&self) -> usize {
		0
	}

	fn shrink_to_fit(&mut self);

	fn buffer(&self) -> &[u8];
}

impl ValidityBuffer for () {
	fn new() -> Self {}

	fn resize_bits(&mut self, _capacity: usize) {}

	fn set_bit_on(&mut self, _index: usize) {}

	fn set_bits_on(&mut self, _start: usize, _num: usize) {}

	fn get(&self, _index: usize) -> bool {
		false
	}

	fn count_ones(&self) -> usize {
		0
	}

	fn memory_size(&self) -> usize {
		0
	}

	fn shrink_to_fit(&mut self) {}

	fn buffer(&self) -> &[u8] {
		&[]
	}
}

impl ValidityBuffer for Vec<u8> {
	fn new() -> Self {
		Self::default()
	}

	fn resize_bits(&mut self, capacity: usize) {
		self.resize(capacity.div_ceil(8), 0);
	}

	fn set_bit_on(&mut self, index: usize) {
		let byte_index = index / 8;
		let bit_offset = index % 8;
		let mask = 1 << bit_offset;
		self[byte_index] |= mask;
	}

	fn set_bits_on(&mut self, start: usize, num: usize) {
		if num == 0 {
			return;
		}

		let byte_start = start / 8;
		let bit_start = start % 8;

		let byte_end = (start + num) / 8;
		let bit_end = (start + num) % 8;

		self[byte_start] |= 0b11111111 << bit_start;

		if byte_start == byte_end {
			return;
		}

		for byte in &mut self[byte_start + 1..byte_end] {
			*byte = 0b11111111;
		}

		if bit_end != 0 {
			self[byte_end] |= 0b11111111 >> (8 - bit_end);
		}
	}

	fn get(&self, index: usize) -> bool {
		let byte_index = index / 8;
		let bit_offset = index % 8;
		let bit = (self[byte_index] >> bit_offset) & 0b1;
		bit == 1
	}

	fn count_ones(&self) -> usize {
		self.iter().map(|&b| b.count_ones() as usize).sum()
	}

	fn memory_size(&self) -> usize {
		self.capacity()
	}

	fn shrink_to_fit(&mut self) {
		self.shrink_to_fit()
	}

	fn buffer(&self) -> &[u8] {
		self
	}
}
