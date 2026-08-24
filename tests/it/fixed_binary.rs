use arbtest::arbtest;
use picoarrow::{
	Error,
	array::{Array, ArrayFixedBinary, NonNullable, Nullable},
};

#[test]
fn simulate_nonnullable() {
	arbtest(|u| {
		let byte_width = u.int_in_range(1..=100)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<&[u8]> = Vec::new();
		let mut arr: ArrayFixedBinary<NonNullable> =
			ArrayFixedBinary::new(byte_width);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let val = u.bytes(byte_width as usize)?;
				expected.push(val);
				arr.push(val).unwrap();
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), *v);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn simulate_nullable() {
	arbtest(|u| {
		let byte_width = u.int_in_range(1..=100)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<&[u8]>> = Vec::new();
		let mut arr: ArrayFixedBinary<Nullable> =
			ArrayFixedBinary::new(byte_width);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 4)? {
				expected.push(None);
				arr.push_null();
			} else if u.ratio(1, 4)? {
				let val = u.bytes(byte_width as usize)?;
				expected.push(Some(val));
				arr.push_some(val).unwrap();
			} else {
				let val = if u.ratio(1, 4)? {
					None
				} else {
					Some(u.bytes(byte_width as usize)?)
				};
				expected.push(val);
				arr.push(val).unwrap();
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), *v);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}

#[test]
fn push_err() {
	let mut arr = ArrayFixedBinary::<NonNullable>::new(10);
	arr.push(&[0; 10]).unwrap();
	assert!(matches!(
		arr.push(&[]).unwrap_err(),
		Error::WrongBinaryAppendLength {
			expected: 10,
			got: 0
		}
	));
}
