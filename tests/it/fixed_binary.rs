use arbtest::arbtest;
use picoarrow::array::{Array, ArrayFixedBinary, NonNullable, Nullable};

#[test]
fn simulate_fixed_binary() {
	arbtest(|u| {
		let byte_width = u.int_in_range(1..=100)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Vec<u8>> = Vec::new();
		let mut arr: ArrayFixedBinary<NonNullable> =
			ArrayFixedBinary::new(byte_width);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let mut val = vec![0u8; byte_width as usize];
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
	.size_min(2u32.pow(20));
}

#[test]
fn simulate_fixed_binary_nullable() {
	arbtest(|u| {
		let byte_width = u.int_in_range(1..=100)?;
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<Vec<u8>>> = Vec::new();
		let mut arr: ArrayFixedBinary<Nullable> =
			ArrayFixedBinary::new(byte_width);

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 3)? {
				expected.push(None);
				arr.push_null();
			} else {
				let mut val = vec![0u8; byte_width as usize];
				for byte in val.iter_mut() {
					*byte = u.arbitrary()?;
				}
				expected.push(Some(val.clone()));
				arr.push(&val).unwrap();
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				match v {
					Some(val) => {
						assert_eq!(
							arr.get(i),
							Some(val.as_slice())
						);
					}
					None => {
						assert!(arr.is_null(i));
						assert!(arr.get(i).is_none());
					}
				}
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(20));
}
