use arbtest::arbtest;
use picoarrow::array::{Array, ArrayBinary, ArrayUtf8, NonNullable};

#[test]
fn simulate_binary() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Vec<u8>> = Vec::new();
		let mut arr: ArrayBinary<NonNullable> = ArrayBinary::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let len = u.int_in_range(0..=64)?;
				let mut val = vec![0u8; len];
				for byte in val.iter_mut() {
					*byte = u.arbitrary()?;
				}
				expected.push(val.clone());
				arr.push(&val).unwrap();
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), v.as_slice());
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
fn simulate_utf8() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<String> = Vec::new();
		let mut arr: ArrayUtf8<NonNullable> = ArrayUtf8::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let val: String = u.arbitrary()?;
				expected.push(val.clone());
				arr.push(&val).unwrap();
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), v.as_str());
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(16));
}
