use arbtest::arbtest;
use picoarrow::array::{Array, ArrayBoolean, NonNullable, Nullable};

use std::ops::ControlFlow;

#[test]
fn simulate() {
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
fn simulate_nullable() {
	arbtest(|u| {
		let steps = u.int_in_range(100..=5_000)?;
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
				arr.push_some(val);
			}

			assert_eq!(arr.len(), expected.len());
			assert_eq!(
				arr.null_count(),
				expected.iter().filter(|v| v.is_none()).count()
			);
			for (i, v) in expected.iter().enumerate() {
				assert_eq!(arr.get(i), *v);
			}
		}
		Ok(())
	})
	.size_min(2u32.pow(16));
}

#[test]
fn simulate_eq_nonnullable() {
	arbtest(|u| {
		let mut arr_a = ArrayBoolean::<NonNullable>::new();
		let mut arr_b = ArrayBoolean::<NonNullable>::new();
		u.arbitrary_loop(Some(10), Some(5000), |u| {
			// XXX: other methods when they are added
			match u.arbitrary::<u8>()? {
				0 => {
					arr_a.clear();
					arr_b.clear();
				}
				1..128 => {
					arr_a.push(true);
					arr_b.push(true);
				}
				128.. => {
					arr_a.push(false);
					arr_b.push(false);
				}
			}
			assert_eq!(arr_a, arr_b);
			Ok(ControlFlow::Continue(()))
		})
	})
	.size_min(2u32.pow(15));
}

#[test]
fn simulate_eq_nullable() {
	arbtest(|u| {
		let mut arr_a = ArrayBoolean::<Nullable>::new();
		let mut arr_b = ArrayBoolean::<Nullable>::new();
		u.arbitrary_loop(Some(10), Some(5000), |u| {
			// XXX: other methods when they are added
			match u.arbitrary::<u8>()? {
				0..64 => {
					arr_a.push_some(true);
					arr_b.push_some(true);
				}
				64..128 => {
					arr_a.push_some(false);
					arr_b.push_some(false);
				}
				128.. => {
					arr_a.push_null();
					arr_b.push_null();
				}
			}
			assert_eq!(arr_a, arr_b);
			Ok(ControlFlow::Continue(()))
		})
	})
	.size_min(2u32.pow(15));
}

#[test]
fn clone() {
	let mut arr = ArrayBoolean::<NonNullable>::new();
	arr.push(true);
	arr.push(false);
	arr.push(true);

	assert_eq!(arr, arr.clone());
}

#[test]
fn eq_edgecase() {
	let mut a = ArrayBoolean::<Nullable>::new();
	for _ in 0..9 {
		a.push_some(false);
	}

	let mut b = ArrayBoolean::<Nullable>::new();
	for _ in 0..8 {
		b.push_some(false);
	}

	assert!(!(a == b));
}
