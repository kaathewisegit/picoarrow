use arbtest::arbtest;
use picoarrow::array::{Array, ArrayBinary, ArrayUtf8, NonNullable, Nullable};

#[test]
fn simulate_binary_nonnullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<&[u8]> = Vec::new();
		let mut arr: ArrayBinary<NonNullable> = ArrayBinary::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let val: &[u8] = u.arbitrary()?;
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
fn simulate_binary_nullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<&[u8]>> = Vec::new();
		let mut arr: ArrayBinary<Nullable> = ArrayBinary::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 4)? {
				expected.push(None);
				arr.push_null();
			} else if u.ratio(1, 4)? {
				let val: &[u8] = u.arbitrary()?;
				expected.push(Some(val));
				arr.push_some(val).unwrap();
			} else {
				let val: Option<&[u8]> = u.arbitrary()?;
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
fn simulate_utf8_nonnullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<&str> = Vec::new();
		let mut arr: ArrayUtf8<NonNullable> = ArrayUtf8::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let val: &str = u.arbitrary()?;
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
fn simulate_utf8_nullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<&str>> = Vec::new();
		let mut arr: ArrayUtf8<Nullable> = ArrayUtf8::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 4)? {
				expected.push(None);
				arr.push_null();
			} else if u.ratio(1, 4)? {
				let val: &str = u.arbitrary()?;
				expected.push(Some(val));
				arr.push_some(val).unwrap();
			} else {
				let val: Option<&str> = u.arbitrary()?;
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
