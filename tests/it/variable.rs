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

#[test]
fn simulate_compare_utf8_nullable() {
	arbtest(|u| {
		let mut arr_a = ArrayUtf8::<Nullable>::new();
		let mut arr_b = ArrayUtf8::<Nullable>::new();
		for s in u.arbitrary_iter::<Option<&str>>()? {
			arr_a.push(s?).unwrap();
			arr_b.push(s?).unwrap();
		}
		assert_eq!(arr_a, arr_b);
		Ok(())
	});
}

#[test]
fn compare_utf8_nullable() {
	let mut arr_a = ArrayUtf8::<Nullable>::new();
	arr_a.push_some("hel").unwrap();
	arr_a.push_some("lo").unwrap();
	for _ in 0..7 {
		arr_a.push_null();
	}

	let mut arr_b = ArrayUtf8::<Nullable>::new();
	arr_b.push_some("hel").unwrap();
	arr_b.push_some("lo").unwrap();
	for _ in 0..6 {
		arr_a.push_null();
	}

	assert_ne!(arr_a, arr_b);
}

#[test]
fn compare_utf8_nonnullable() {
	let mut arr_a = ArrayUtf8::<NonNullable>::new();
	arr_a.push("hel").unwrap();
	arr_a.push("lo").unwrap();

	let mut arr_b = ArrayUtf8::<NonNullable>::new();
	arr_b.push("hell").unwrap();
	arr_b.push("o").unwrap();

	assert_ne!(arr_a, arr_b);
}

#[test]
fn clone_binary() {
	let mut arr = ArrayBinary::<NonNullable>::new();
	arr.push(b"hello").unwrap();
	arr.push(b"there").unwrap();

	assert_eq!(arr, arr.clone());

	let mut arr = ArrayBinary::<Nullable>::new();
	arr.push_some(b"hello").unwrap();
	arr.push_null();
	arr.push_some(b"there").unwrap();

	assert_eq!(arr, arr.clone());
}

#[test]
fn clone_utf8() {
	let mut arr = ArrayUtf8::<NonNullable>::new();
	arr.push("hello").unwrap();
	arr.push("there").unwrap();

	assert_eq!(arr, arr.clone());

	let mut arr = ArrayUtf8::<Nullable>::new();
	arr.push_some("hello").unwrap();
	arr.push_null();
	arr.push_some("there").unwrap();

	assert_eq!(arr, arr.clone());
}
