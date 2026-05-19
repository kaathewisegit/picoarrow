use arbtest::arbtest;
use picoarrow::array::{Array, ArrayBoolean, NonNullable, Nullable};

#[test]
fn simulate_boolean() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<bool> = Vec::new();
		let mut arr: ArrayBoolean<NonNullable> = ArrayBoolean::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else {
				let val: bool = u.arbitrary()?;
				expected.push(val);
				arr.push(val);
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), *v);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
fn simulate_boolean_nullable() {
	arbtest(|u| {
		let steps = u.int_in_range(0..=5_000)?;
		let mut expected: Vec<Option<bool>> = Vec::new();
		let mut arr: ArrayBoolean<Nullable> = ArrayBoolean::new();

		for _ in 0..steps {
			if u.ratio(1, 100)? {
				expected.clear();
				arr.clear();
			} else if u.ratio(1, 3)? {
				expected.push(None);
				arr.push_null();
			} else {
				let val: bool = u.arbitrary()?;
				expected.push(Some(val));
				arr.push(val);
			}

			assert_eq!(arr.len(), expected.len());
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), *v);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(16));
}
